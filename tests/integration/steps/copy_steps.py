from behave import when
from botocore.exceptions import ClientError


def _split(path):
    bucket, key = path.split("/", 1)
    return bucket, key


@when('I copy object "{src}" to "{dst}"')
def step_copy_object(context, src, dst):
    src_bucket, src_key = _split(src)
    dst_bucket, dst_key = _split(dst)
    context.copy_response = context.s3.copy_object(
        Bucket=dst_bucket,
        Key=dst_key,
        CopySource=f"{src_bucket}/{src_key}",
    )


@when('I try to copy object "{src}" to "{dst}"')
def step_try_copy_object(context, src, dst):
    src_bucket, src_key = _split(src)
    dst_bucket, dst_key = _split(dst)
    try:
        context.s3.copy_object(
            Bucket=dst_bucket,
            Key=dst_key,
            CopySource=f"{src_bucket}/{src_key}",
        )
        context.last_error = None
    except ClientError as e:
        context.last_error = e
