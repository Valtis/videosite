use std::cmp::min;
use std::env;
use std::io;

use std::process::Stdio;

use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedPart, CompletedMultipartUpload};
use aws_sdk_sqs::Client;
use tokio::io::AsyncReadExt;
use tracing_subscriber::filter;
use serde_json;

use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use tokio::select;
use tokio::process::Command;

use futures_util::StreamExt;

use reqwest;

use shlex;

#[allow(dead_code)]
struct MetadataEvent {
    pub message: MetadataMessage,
    pub receipt_handle: String,
}


#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct MetadataMessage {
    pub presigned_url: String,
    pub object_name: String,
    pub file_type: FileType,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct AudioData {
    pub duration: f64, // Duration in seconds
    pub bitrate: u32, // Bitrate in kbps
    pub sample_rate: u32, // Sample rate in Hz
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct VideoData {
    pub duration: f64, // Duration in seconds
    pub width: u32,   // Width in pixels
    pub height: u32,  // Height in pixels
    pub bitrate: u32, // Bitrate in kbps
    pub frame_rate: f64, // Frame rate in frames per second
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
enum FileType {
    Video{ video: VideoData, audio: Option<AudioData>}, 
}

#[allow(dead_code)]
struct TranscodingOptions {
    width: u32,
    height: u32,
    frame_rate: u32,
    video_bitrate: u32,
}

const VIDEO_CODEC: &'static str = "libx264"; 
const AUDIO_CODEC: &'static str = "aac";
const SEGMENT_LENGTH_SECONDS: u32 = 5; // Length of each segment in seconds

const AUDIO_BITRATE: u32 = 128*1024;

const TRANSCODING_OPTIONS_144P: TranscodingOptions = TranscodingOptions {
    width: 256,
    height: 144,
    frame_rate: 30,
    video_bitrate: 250*1024,
};

const TRANSCODING_OPTIONS_270P: TranscodingOptions = TranscodingOptions {
    width: 480,
    height: 270,
    frame_rate: 30,
    video_bitrate: 750*1024,
};

const TRANSCODING_OPTIONS_480P: TranscodingOptions = TranscodingOptions {
    width: 854,
    height: 480,
    frame_rate: 30,
    video_bitrate: (2.5*1024.0*1024.0) as u32, // 2.5 Mbps
};

const TRANSCODING_OPTIONS_720P: TranscodingOptions = TranscodingOptions {
    width: 1280,
    height: 720,
    frame_rate: 60,
    video_bitrate: 5*1024*1024,
};

const TRANSCODING_OPTIONS_1080P: TranscodingOptions = TranscodingOptions {
    width: 1920,
    height: 1080,
    frame_rate: 60,
    video_bitrate: 8*1024*1024,
};

const RESOURCE_FOLDER_NAME: &'static str = "resource";

fn get_object_path(object_name: &str) -> String {
    format!("{}/{}", RESOURCE_FOLDER_NAME, object_name)
}

fn s3_bucket() -> String {
    env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_level(true)
        .with_max_level(filter::LevelFilter::INFO)
        .init();
    tracing::info!("Starting video transcoding service");

    let queue_url = env::var("VIDEO_PROCESSING_QUEUE_URL").expect("VIDEO_PROCESSING_QUEUE_URL not set");
    let client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);

    loop {
        let video_metadata_opt = receive_video_metadata_event(&client, &queue_url).await
            .unwrap_or_else(|err| {
                tracing::error!("Error receiving metadata notification: {}", err);
                None
            });

        if let Some(video_metadata) = video_metadata_opt {
            tracing::info!("Received video metadata for {}: {:?}", video_metadata.message.object_name, video_metadata.message.file_type);

            if let Ok(_) = transcode_video(&video_metadata.message).await {
                tracing::info!("Video transcoding was completed successfully for {}", video_metadata.message.object_name);
                queue_resource_processing_completed_event(&video_metadata.message.object_name).await;
            } else {
                tracing::error!("Video transcoding failed for {}", video_metadata.message.object_name);
                queue_resource_status_update_event(&video_metadata.message.object_name, "failed").await;
            }
            
            delete_message(&client, &queue_url, &video_metadata.receipt_handle)
                .await
                .unwrap_or_else(|err| {
                    tracing::error!("Failed to delete message: {}", err);
            });
            tracing::info!("Message deleted successfully");
        }
        // Sleep for a while before checking the queue again
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

}


async fn receive_video_metadata_event(client: &Client, queue_url: &str) -> Result<Option<MetadataEvent>, aws_sdk_sqs::Error> {
    let rcv_message_output = match client
        .receive_message()
        .queue_url(queue_url)
        .max_number_of_messages(1)
        .visibility_timeout(3600*6) // video processing can take a long time, so we give it 6 hours
        .send()
        .await {
            Ok(output) => output,
            Err(err) => {
                tracing::error!("Failed to receive message from SQS queue: {}", err);
                return Ok(None);    
            }
        };

    
    for message in rcv_message_output.messages.unwrap_or_default() {

        let body = match message.body {
            Some(body) => body,
            None => {
                tracing::warn!("Received message with no body, skipping.");
                continue;
            }
        };

        let metadata_message: MetadataMessage = match serde_json::from_str(&body){
            Ok(msg) => msg,
            Err(err) => {
                tracing::error!("Failed to parse message body as JSON: {}", err);
                continue;
            }
        };

        return Ok(Some(MetadataEvent {
            message: metadata_message,
            receipt_handle: message.receipt_handle.unwrap_or_default(),
        }));         

    }

    Ok(None)
}



async fn transcode_video(
    msg: &MetadataMessage,
) -> Result<(), &'static str> {
    tracing::info!("Transcoding video: {}", msg.object_name);
    let FileType::Video { video, audio } = &msg.file_type;

    tracing::info!("Video stats: {:?}", video);
    if let Some(audio_data) = audio {
        tracing::info!("Audio stats: {:?}", audio_data);
    } else {
        tracing::info!("No audio data available.");
    }

    // create directory at /transcoding<object_name> to store the transcoded files
    let workdir = format!("/transcoding/{}", msg.object_name);
    std::fs::create_dir_all(&workdir).map_err(|_| "Failed to create work directory")?;

    let input_file_path = download_input_file(&msg.presigned_url, &msg.object_name, &workdir).await;

    let ffmpeg_str = construct_video_transcoding_options_for_ffmpeg(
        video,
        audio,
        &input_file_path);

    tracing::info!("FFMPEG string: ffmpeg {}", 
        ffmpeg_str    
    );
    run_ffmpeg(&workdir, &ffmpeg_str)
        .await.map_err(|_| "FFMPEG process failed")?;

    // delete the input file after processing, as we will not need it anymore, and we will upload the transcoded files to S3
    // and the presence of this file would force us to filter it out
    std::fs::remove_file(&input_file_path).expect("Failed to delete input file after processing");
    tracing::info!("Input file {} deleted after processing", input_file_path);

    
    transfer_files_to_s3(&workdir, &msg.object_name)
        .await
        .map_err(|_| "Failed to transfer files to S3")?;
    
    Ok(())
}

/// Downloads a file from a presigned URL and saves it to a temporary location
/// 
/// This is due to observed behavior, where using presigned URLs with ffmpeg
/// occasionally fails due to IO errors, and downloading the file first
/// has been observed to be more reliable.
/// 
/// # Arguments
/// * `presigned_url` - The presigned URL to download the file from
/// * `object_name` - The name to save the downloaded file as
/// # Returns
/// The path to the downloaded file
async fn download_input_file(presigned_url: &str, object_name: &str, workdir: &str) -> String {
    let output_path = format!("{}/{}", workdir, object_name);
    let mut output_file = tokio::fs::File::create(&output_path).await.expect("Failed to create output file");

    let mut stream = reqwest::get(presigned_url)
        .await
        .expect("Failed to download file")
        .bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.expect("Failed to read chunk");
        output_file.write_all(&chunk).await.expect("Failed to write chunk to file");
    }

    output_file.flush().await.expect("Failed to flush output file");
    
  
    output_path
}


/// Constructs the ffmpeg command line options for transcoding a video file
/// 
/// This function generates a ffmpeg command that:
/// * Splits the input video into multiple streams based on the specified transcoding options
/// * Applies scaling and frame rate adjustments to each stream
/// * Maps the video and audio streams to the appropriate codecs and bitrates
/// * Outputs the transcoded video in HLS format with independent segments
/// Generated video streams will depend on the input video's width 
/// 
/// # Arguments
/// * `object_name` - The name of the output file (used for naming segments)
/// * `video_stats` - Statistics about the video to be transcoded
/// * `audio_stats` - Optional statistics about the audio stream (if available)
/// * `input_file` - The path to the input video file
/// 
/// # Returns
/// A string containing the ffmpeg command line options for transcoding the video
/// 
fn construct_video_transcoding_options_for_ffmpeg(
    video_stats: &VideoData,
    audio_stats: &Option<AudioData>,
    input_file: &str,
) -> String  {


    let aspect_ration = video_stats.width as f64 / video_stats.height as f64;

    let mut encodings_to_use = vec![];

    encodings_to_use.push(&TRANSCODING_OPTIONS_144P);

    if video_stats.width >= TRANSCODING_OPTIONS_270P.width {
        encodings_to_use.push(&TRANSCODING_OPTIONS_270P);
    }

    if video_stats.width >= TRANSCODING_OPTIONS_480P.width {
        encodings_to_use.push(&TRANSCODING_OPTIONS_480P);
    }

    if video_stats.width >= TRANSCODING_OPTIONS_720P.width {
        encodings_to_use.push(&TRANSCODING_OPTIONS_720P);
    }

    if video_stats.width >= TRANSCODING_OPTIONS_1080P.width {
        encodings_to_use.push(&TRANSCODING_OPTIONS_1080P);
    }


    let mut filter_strs = vec![];
    let mut map_strings = vec![];
    for (i, transcoding_options) in encodings_to_use.iter().enumerate() {
        let (filter_str, map_string) = construct_transcoding_options_with_parameters(
            i as u32,
            video_stats,
            aspect_ration,
            transcoding_options,
        );

        filter_strs.push(filter_str);
        map_strings.push(map_string);
    }

    let stream_count = filter_strs.len();

    let mut split_str = String::new();

    for i in 0..filter_strs.len() {
            split_str += &format!("[v{i}in]")
    }

    let mut ffmpeg_args = format!("-i {input_file} -filter_complex \"[0:v]split={stream_count}{split_str};",
        input_file=input_file,
        stream_count=stream_count,
        split_str=split_str        
    );

    for filter_str in filter_strs {
        ffmpeg_args += &filter_str;
    }
    ffmpeg_args += "\" ";

    for map_str in map_strings {
        ffmpeg_args += &map_str;
    }

    if let Some(audio_stats) = audio_stats {
        
        for i in 0..stream_count {
            ffmpeg_args += &format!(
                " -map a:0 -c:a:{i} {audio_codec} -b:a:{i} {audio_bitrate}k -ac:{i} 2",
                i=i,
                audio_codec=AUDIO_CODEC,
                audio_bitrate=min(AUDIO_BITRATE, audio_stats.bitrate) / 1024,
            );
        }
    }

    let mut stream_map_str = String::new();

    for i in 0..stream_count {
        if i > 0 {
            stream_map_str += " ";
        }
        if let Some(_) = audio_stats {
            stream_map_str += &format!("v:{i},a:{i}", i=i);
        } else {
            stream_map_str += &format!("v:{i}", i=i);
        }
    }

    ffmpeg_args += &format!(
        " -f hls -hls_playlist_type vod -hls_flags independent_segments -hls_segment_type mpegts \
         -hls_segment_filename \"stream_%v/data%04d.ts\" \
         -master_pl_name master.m3u8 -var_stream_map \"{stream_map_str}\" \
         stream_%v/playlist.m3u8",
        stream_map_str=stream_map_str
    );

    ffmpeg_args
}


fn construct_transcoding_options_with_parameters(
    id: u32,
    video_stats: &VideoData,
    aspect_ratio: f64,
    transcoding_options: &TranscodingOptions,
) -> (String, String) {
    //  -map "[v1out]" -c:v:0 libx264 -b:v:0 2000k -maxrate:v:0 3000k -bufsize:v:0 4000k -g 150 -keyint_min 150 -hls_time 150  \
    let width = min(transcoding_options.width, video_stats.width);
    let height = (transcoding_options.width as f64 / aspect_ratio) as u32;


    let target_fps = min(video_stats.frame_rate as u32, transcoding_options.frame_rate);
        
    // if we have a video with fps above 45 (probably 60fps), grant a bitrate boost of 1.5x 
    let target_video_bitrate = (min(transcoding_options.video_bitrate, video_stats.bitrate) as f64 * if target_fps >= 45 { 1.5 } else { 1.0 }) as u32;

    // allow bitrate exceed target by 50% for maximum bitrate if the codec thinks it is necessary - should help with scenes with high motion
    let maximum_bitrate = (target_video_bitrate as f64 * 1.5) as u32;
    // buffer twice the size of the target bitrate, to prevent overly strict bitrate limits that may hurt quality
    let buffer_size = (target_video_bitrate as f64 * 2.0) as u32; 
 
    let filter_str = format!("[v{id}in]scale={width}:{height}[v{id}fps];[v{id}fps]fps={target_fps}[v{id}out];",
        id=id,
        width=width,
        height=height,
    );


    /*
        For each quality stream, we will map the video stream to the codec and set the bitrate parameters.
        * codec (-c:v): libx264
        * target bitrate (-b:v): minimum of video bitrate and transcoding options bitrate
        * maximum bitrate (-maxrate): 1.5x target bitrate
        * buffer size (-bufsize): 2x target bitrate
        * group of pictures size in frames (-g): 5 seconds (keyframe interval)
        * keyframe interval in frames (-keyint_min): target_fps * 5 seconds
        * No scene change detection, force keyframes every 5 seconds (-sc_threshold 0)
        * Another setting to REALLY force keyframes every 5 seconds (-force_key_frames "expr:gte(t,n_forced*5)")
        * HLS segment length in seconds (-hls_time): 5 seconds`
        * Metadata will be stripped from the output segments (-map_metadata -1)

        Due to small differences and codec decisions, keyframes were observed to not necessarily be placed exactly at the 5 second mark,
        hence why there are so many settings for telling the codec that yes, we want keyframes every 5 seconds.
     */
    let map_string = format!(
        " -map \"[v{id}out]\" -c:v:{id} {codec} -b:v:{id} {target_video_bitrate}k -maxrate:v:{id} {maximum_bitrate}k \
        -bufsize:v:{id} {buffer_size}k -g {keyframe_interval} -keyint_min {keyframe_interval} -sc_threshold 0 \
        -force_key_frames \"expr:gte(t,n_forced*5)\" -hls_time {segment_length_seconds} -map_metadata -1",
        id=id,
        codec=VIDEO_CODEC,
        target_video_bitrate=target_video_bitrate / 1024,
        maximum_bitrate=maximum_bitrate / 1024,
        buffer_size=buffer_size / 1024,
        keyframe_interval=target_fps * SEGMENT_LENGTH_SECONDS,
        segment_length_seconds=SEGMENT_LENGTH_SECONDS,
    );

    (filter_str, map_string)

}


async fn run_ffmpeg(workdir: &str, ffmpeg_string: &str) -> Result<(), io::Error> {

    let args = shlex::split(ffmpeg_string).expect("Failed to split ffmpeg command string");

    tracing::info!("Running ffmpeg ");
    let mut child = Command::new("ffmpeg")
        .args(args)
        .current_dir(workdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start FFMPEG process");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut stderr_lines = vec![];

    loop {
        select! {
            line = stdout_reader.next_line() => {
                match line? {
                    Some(line) => tracing::info!("{}", line),
                    None => break, // EOF on stdout
                }
            }
            line = stderr_reader.next_line() => {
                match line? {
                    Some(line) => stderr_lines.push(line),
                    None => break, // EOF on stderr
                }
            }
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        let stderr_output = stderr_lines.join("\n");
        tracing::error!("ffmpeg command failed with status: {:?}\nStderr: {}", status, stderr_output);
        return Err(io::Error::new(io::ErrorKind::Other, "FFMPEG command failed"));
    }


    Ok(())
   
}


/// Create a foler using the file name (which should be unique) and upload the generated HLS files to this folder
/// 
/// # Arguments
/// * `workdir` - The directory where the transcoded files are stored
/// * `object_name` - The name of the file to be uploaded (used for naming the S3 object)
///
/// 
async fn transfer_files_to_s3(workdir: &str, object_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = aws_config::load_from_env().await;
    let s3_client = S3Client::new(&config);
    tracing::info!("Creating S3 client");
    let s3_client = if let Ok(var) = env::var("USE_PATH_STYLE_BUCKETS") {
        if var.to_lowercase() == "true" {
            let config_builder = s3_client.config().clone().to_builder();
            S3Client::from_conf(config_builder.force_path_style(true).build())
        } else {
            s3_client
        }
    } else {
        s3_client
    };

    let files_to_upload = list_files_for_uploading(workdir).await?;

    for file_path in files_to_upload {
        upload_file(&s3_client, object_name, workdir, file_path).await?;
    }


    Ok(())
}

async fn list_files_for_uploading(workdir: &str) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    // find the files in workdir as well as the subdirectories. We have at most one level of subdirectories.
    let mut files_to_upload = vec![];
    let mut entries = tokio::fs::read_dir(workdir).await.expect("Failed to read workdir");

    while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
        let path = entry.path();
        if path.is_file() {
            files_to_upload.push(path);
        } else if path.is_dir() {
            // If it's a directory, we assume it contains the HLS segments and playlists
            let mut sub_entries = tokio::fs::read_dir(&path).await.expect("Failed to read subdirectory");
            while let Some(sub_entry) = sub_entries.next_entry().await.expect("Failed to read sub-entry") {
                files_to_upload.push(sub_entry.path());
            }
        }
    }
    
    Ok(files_to_upload)
}

async fn upload_file(client: &S3Client, object_name: &str, workdir: &str, path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {

    // create the S3 object key: Strip the workdir prefix and replace with the file name
    // this should result into key like "abcd/master.m3u8"
    let object_name = format!(
        "{}/{}",
        object_name,
        path.strip_prefix(workdir)
            .expect("Failed to strip workdir prefix")
            .to_str()
            .expect("Failed to convert path to str")
    );

    tracing::info!("Uploading file {} to S3 with object name: {}", path.display(), object_name);

   
    let multi_part_upload = client.create_multipart_upload()
        .bucket(s3_bucket()) 
        .key(get_object_path(&object_name))
        .send()
        .await
        .expect("Failed to create multipart upload");

    let upload_id = multi_part_upload.upload_id().expect("Upload ID not found");


    let mut part_number = 1;
    let mut completed_parts: Vec<CompletedPart> = Vec::new();
    

    const CHUNK_SIZE: usize = 5 * 1024 * 1024; // 5 MB, S3 minimum part size
    const BYTES_TO_READ: usize = 5*1024; // 5 kb

    let mut read_buffer: [u8; BYTES_TO_READ] = [0; BYTES_TO_READ];
    let mut buffer = vec![];


    let file = tokio::fs::File::open(&path).await.expect("Failed to open file for uploading");
    let mut reader = BufReader::new(file);

    loop {
        let read_bytes = reader.read(&mut read_buffer).await.expect("Failed to read from file");
        if read_bytes == 0 {
            break; // EOF reached
        }

        buffer.extend_from_slice(&read_buffer[..read_bytes]);

        if buffer.len() > CHUNK_SIZE {
            upload_chunk(&client, buffer, &object_name, upload_id, part_number, &mut completed_parts).await;
            buffer = vec![];
            part_number += 1;
        }
    }

    // upload any remaining data in the buffer
    if !buffer.is_empty() {
        upload_chunk(&client, buffer, &object_name, upload_id, part_number, &mut completed_parts).await;
    }


    let completed_multipart_upload: CompletedMultipartUpload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    client.complete_multipart_upload()
        .bucket(s3_bucket())
        .key(get_object_path(&object_name))
        .multipart_upload(completed_multipart_upload)
        .upload_id(upload_id)
        .send()
        .await
        .expect("Failed to complete multipart upload");

    Ok(())
}
        
async fn upload_chunk(
    client: &S3Client,
    buffer: Vec<u8>,
    object_name: &str,
    upload_id: &str,
    part_number: i32,
    completed_parts: &mut Vec<CompletedPart>
) {
    let bytes = ByteStream::from(buffer);
    let part: aws_sdk_s3::operation::upload_part::UploadPartOutput = client.upload_part()
        .bucket(s3_bucket()) 
        .key(get_object_path(&object_name))
        .part_number(part_number)
        .upload_id(upload_id)
        .body(bytes.into())
        .send()
        .await
        .expect("Failed to upload part");

    completed_parts.push(CompletedPart::builder()
        .part_number(part_number)
        .e_tag(part.e_tag().unwrap_or("not set").to_string())
        .build());
}

async fn queue_resource_processing_completed_event(object_name: &str) {
    let sqs_client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);
    let queue_url = env::var("RESOURCE_STATUS_QUEUE_URL").expect("RESOURCE_STATUS_QUEUE_URL not set");

    let json_msg = serde_json::json!({
        "object_name": object_name,
        "status": "processed",
    }).to_string(); 

    tracing::info!("Sending resource processing completed message {} to SQS queue: {}", json_msg, queue_url);

    sqs_client.send_message()
        .queue_url(queue_url)
        .message_body(json_msg)
        .send()
        .await
        .expect("Failed to send resource status update message to SQS");
}

async fn queue_resource_status_update_event(object_name: &str, status: &str) {
    let sqs_client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);
    let queue_url = env::var("RESOURCE_STATUS_QUEUE_URL").expect("RESOURCE_STATUS_QUEUE_URL not set");

    let json_msg = serde_json::json!({
        "object_name": object_name,
        "status": status
    }).to_string(); 

    tracing::info!("Sending resource status update message {} to SQS queue: {}", json_msg, queue_url);

    sqs_client.send_message()
        .queue_url(queue_url)
        .message_body(json_msg)
        .send()
        .await
        .expect("Failed to send resource status update message to SQS");
}

async fn delete_message(client: &Client, queue_url: &str, receipt_handle: &str) -> Result<(), aws_sdk_sqs::Error> {
    client
        .delete_message()
        .queue_url(queue_url)
        .receipt_handle(receipt_handle)
        .send()
        .await?;

    tracing::info!("Message deleted successfully");
    Ok(())
}