//! Read-only S3 check against the configured bucket.
//!
//! Confirms that the AWS credentials in the environment can see the bucket,
//! that the bucket lives in `AWS_REGION` (so generated media URLs match the
//! existing `Media.url` scheme), and that every object referenced by the
//! `Media` collection actually exists. Nothing is uploaded or deleted.
//!
//! ```text
//! cargo run --example verify_s3
//! ```

use oximod::OxiClient;
use portfolio::config::Config;
use portfolio::repositories;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    let aws_config = aws_config::from_env()
        .region(aws_config::Region::new(config.aws_region.clone()))
        .load()
        .await;
    let s3 = aws_sdk_s3::Client::new(&aws_config);

    println!(
        "bucket: {} (expected region {})",
        config.s3_bucket, config.aws_region
    );
    s3.head_bucket().bucket(&config.s3_bucket).send().await?;
    println!("head_bucket: OK");

    let location = s3
        .get_bucket_location()
        .bucket(&config.s3_bucket)
        .send()
        .await?;
    let region = location
        .location_constraint()
        .map(|c| c.as_str().to_string())
        .unwrap_or_else(|| "us-east-1".to_string());
    println!(
        "bucket location: {region} (matches AWS_REGION: {})",
        region == config.aws_region
    );

    OxiClient::init_global(config.database_url.clone()).await?;
    let media = repositories::media::all().await?;
    let mut missing = 0usize;
    for m in &media {
        match s3
            .head_object()
            .bucket(&config.s3_bucket)
            .key(&m.file_key)
            .send()
            .await
        {
            Ok(head) => println!(
                "  OK   {} ({} bytes, content-type {:?})",
                m.file_key,
                head.content_length().unwrap_or(0),
                head.content_type().unwrap_or("-")
            ),
            Err(err) => {
                missing += 1;
                println!("  MISSING {} ({})", m.file_key, err.into_service_error());
            }
        }
    }
    println!(
        "media objects checked: {}, missing in S3: {missing}",
        media.len()
    );
    Ok(())
}
