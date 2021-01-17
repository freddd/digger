extern crate rusoto_core;
extern crate rusoto_s3;
extern crate serde;

use log::{debug, error, info};
use rusoto_core::{request::BufferedHttpResponse, Region, RusotoError};
use rusoto_s3::{
    DeleteObjectRequest, HeadBucketError, HeadBucketRequest, ListObjectsRequest, PutObjectRequest,
    S3Client, S3,
};

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct AWSs3 {
    region: Region,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Error {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "Endpoint")]
    pub endpoint: String,
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "RequestId")]
    pub request_id: String,
    #[serde(rename = "HostId")]
    pub host_id: String,
}

impl Error for S3Error {}
impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {}", self.message, self.endpoint)
    }
}

#[derive(Debug, Clone)]
struct UnknownError(u16);
impl Error for UnknownError {}
impl fmt::Display for UnknownError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error, status code: {}", self.0)
    }
}

// https://labs.detectify.com/2017/07/13/a-deep-dive-into-aws-s3-access-controls-taking-full-control-over-your-assets/?utm_source=blog&utm_campaign=s3_buckets
impl AWSs3 {
    pub fn new(region: &str) -> AWSs3 {
        AWSs3 {
            region: region.parse::<Region>().unwrap(),
        }
    }

    fn file_name(&self) -> String {
        return String::from("really-long-name-that-is-definitely-not-used.txt");
    }

    // TODO: Clean up and refactor
    pub async fn scan(&self, buckets: Vec<&str>) {
        for bucket in buckets {
            let head = self.head(bucket).await;

            match head {
                Ok(exists) => {
                    if exists {
                        debug!("bucket={}: exists", bucket);

                        let list = self.list(bucket).await;
                        match list {
                            Ok(_) => {
                                info!("bucket={}: successfully listed objects", bucket);
                            }
                            Err(err) => {
                                error!("error listing objects, bucket={}: {}", bucket, err);
                            }
                        }

                        let upload = self.upload(bucket).await;

                        match upload {
                            Ok(_) => {
                                info!("bucket={}: successfully uploaded a file", bucket);

                                let remove = self.remove(bucket).await;
                                match remove {
                                    Ok(_) => {
                                        info!("bucket={}: successfully removed file", bucket);
                                    }
                                    Err(err) => {
                                        error!(
                                            "error when removing file, bucket={}: {:#?}",
                                            bucket, err
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                error!("error uploading file, bucket={}: {}", bucket, err);
                            }
                        }
                    } else {
                        error!("bucket={}: does not exist", bucket);
                    }
                }
                Err(err) => {
                    error!(
                        "error looking up bucket, bucket={}: {:?}",
                        bucket,
                        err.to_string()
                    );
                }
            }
        }
    }
    async fn upload(&self, bucket: &str) -> Result<(), Box<dyn Error>> {
        let req = PutObjectRequest {
            bucket: String::from(bucket),
            key: self.file_name(),
            body: Some(self.string_to_body(String::from("it should not be possible to do this!"))),
            ..Default::default()
        };

        self.handle_response(S3Client::new(self.region.clone()).put_object(req).await)
    }
    async fn list(&self, bucket: &str) -> Result<(), Box<dyn Error>> {
        let req = ListObjectsRequest {
            bucket: String::from(bucket),
            ..Default::default()
        };

        self.handle_response(S3Client::new(self.region.clone()).list_objects(req).await)
    }

    async fn remove(&self, bucket: &str) -> Result<(), Box<dyn Error>> {
        let req = DeleteObjectRequest {
            bucket: String::from(bucket),
            key: self.file_name(),
            ..Default::default()
        };

        self.handle_response(S3Client::new(self.region.clone()).delete_object(req).await)
    }

    async fn head(&self, bucket: &str) -> Result<bool, Box<dyn Error>> {
        let req = HeadBucketRequest {
            bucket: String::from(bucket),
        };

        match S3Client::new(Region::default()).head_bucket(req).await {
            Ok(_) => Ok(true),
            Err(RusotoError::Service(HeadBucketError::NoSuchBucket(_))) => {
                debug!("status is NoSuchBucket");
                Ok(false)
            }
            Err(RusotoError::Unknown(BufferedHttpResponse { status, .. })) => {
                if status.as_u16() == 404 {
                    Ok(false)
                } else if status.as_u16() == 301 {
                    Ok(true)
                } else {
                    Err(Box::new(UnknownError(status.as_u16())))
                }
            }
            Err(err) => Err(err.into()),
        }
    }

    fn handle_response<T, E: std::error::Error + 'static>(
        &self,
        result: Result<T, RusotoError<E>>,
    ) -> Result<(), Box<dyn Error>> {
        match result {
            Ok(_) => Ok(()),
            Err(RusotoError::Unknown(BufferedHttpResponse { status, body, .. })) => {
                if status.as_u16() == 301 {
                    let b = std::str::from_utf8(&body).unwrap();
                    let err: S3Error = serde_xml_rs::from_str(b).unwrap();
                    Err(err.into())
                } else {
                    Err(UnknownError(status.as_u16()).into())
                }
            }
            Err(err) => Err(err.into()),
        }
    }

    fn string_to_body(&self, s: String) -> rusoto_s3::StreamingBody {
        s.into_bytes().into()
    }
}
