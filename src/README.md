# DIGGER

## Experimental - needs lots of refactoring

Identifies bucket/container misconfigurations.

## S3 (AWS)
> Requires AWS_ACCESS_KEY and AWS_SECRET_KEY to be set.
Tries to list objects, upload an object and remove the uploaded object.
```bash
USAGE:
    digger s3 --region=<region> bucket1 bucket2 bucketN
```

## GCS (GCP)
> Requires GOOGLE_APPLICATIONS_CREDENTIALS to be set.
Uses the testPermissions endpoint to find out if it's possible to get, update, delete objects and get/set IAM policy.
```bash
USAGE:
    digger gcs bucket1 bucket2 bucketN
```

## STORAGE (Azure)
> Runs as unauthenticated - upload and delete is not implemented

```bash
USAGE:
    digger storage --account=<account> container1 container2 containerN
```
