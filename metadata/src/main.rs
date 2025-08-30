use std::env;

use std::process::Stdio;

use aws_sdk_sqs::Client;
use tracing_subscriber::filter;
use serde_json;

use tokio::io::AsyncBufReadExt;
use tokio::select;
use tokio::process::Command;
use tokio::io::BufReader;
use tokio::io;

#[derive(Debug, serde::Deserialize)]
struct ScanMessage {
    pub presigned_url: String,
    pub object_name: String,
}

struct ScanEvent {
    pub message: ScanMessage,
    pub receipt_handle: String,
}

#[derive(Debug, serde::Serialize)]
struct AudioData {
    pub duration: f64, // Duration in seconds
    pub bitrate: u32, // Bitrate in kbps
    pub sample_rate: u32, // Sample rate in Hz
}

#[derive(Debug, serde::Serialize)]
struct VideoData {
    pub duration: f64, // Duration in seconds
    pub width: u32,   // Width in pixels
    pub height: u32,  // Height in pixels
    pub bitrate: u32, // Bitrate in kbps
    pub frame_rate: f64, // Frame rate in frames per second
}

#[derive(Debug, serde::Serialize)]
struct ImageData {
    pub width: u32,   // Width in pixels
    pub height: u32,  // Height in pixels
}

#[derive(Debug, serde::Serialize)]
enum FileType {
    Video{ video: VideoData, audio: Option<AudioData>}, 
    Audio{ audio: AudioData},
    Image{ image: ImageData },
    Other,
}

impl FileType {
    fn as_str(&self) -> &str {
        match self {
            FileType::Video { .. } => "video",
            FileType::Audio { .. } => "audio",
            FileType::Image { .. } => "image",
            FileType::Other => "other",
        }
    }

    fn is_media_type(&self) -> bool {
        matches!(self, FileType::Video { .. } | FileType::Audio { .. } | FileType::Image { .. })
    }
}


#[derive(Debug, serde::Deserialize)]
struct MediaInfo {
    pub media: Media,
}


#[derive(Debug, serde::Deserialize)]
struct Media {
    pub track: Vec<Track>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "@type")]
enum Track {
    #[serde(rename = "General")]
    General {
        #[serde(rename = "FileSize")]
        file_size: String,
        // Non-mediafiles generally have no sensible metadata.
        #[serde(rename = "Duration")]
        duration: Option<String>,
        #[serde(rename = "Format")]
        format: Option<String>,
        #[serde(rename = "VideoCount")]
        video_count: Option<String>,
        #[serde(rename = "AudioCount")]
        audio_count: Option<String>,
        #[serde(rename = "ImageCount")]
        image_count: Option<String>,
        #[serde(rename = "OtherCount")]
        other_count: Option<String>
    },
    #[serde(rename = "Video")]
    Video {
        #[serde(rename = "Width")]
        width: String, // everything is a string in MediaInfo
        #[serde(rename = "Height")]
        height: String,
        #[serde(rename = "Duration")]
        duration: String,
        #[serde(rename = "FrameRate")]
        frame_rate: String,
        #[serde(rename = "BitRate")]
        bitrate: String,
    },
    #[serde(rename = "Audio")]
    Audio {
        #[serde(rename = "Duration")]
        duration: String,
        #[serde(rename = "BitRate")]
        bitrate: String,
        #[serde(rename = "SamplingRate")]
        sample_rate: String,
    },
    #[serde(rename = "Image")]
    Image {
        #[serde(rename = "Width")]
        width: String,
        #[serde(rename = "Height")]
        height: String,  
    },
    #[serde(other)]
    Other, // Catch-all for any other track types
}



#[tokio::main]
async fn main() {

    tracing_subscriber::fmt()
        .with_level(true)
        .with_max_level(filter::LevelFilter::INFO)
        .init();


    let queue_url = env::var("VIRUS_SCAN_QUEUE_URL").expect("VIRUS_SCAN_QUEUE_URL not set");
    let client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);

    loop {
       let scan_event_opt = receive_virus_scan_completed_notification(&client, &queue_url).await
            .unwrap_or_else(|err| {
                tracing::error!("Error receiving upload notification: {}", err);
                None
            });

        if let Some(scan_event) = scan_event_opt {
            tracing::info!("Received scan event for file: {}", scan_event.message.object_name);

            let file_type = discover_filetype_and_metadata(&scan_event.message.presigned_url).await
                .unwrap_or_else(|err| {
                    tracing::error!("Error discovering file type: {}", err);
                    FileType::Other // Default to Other if there's an error
                });

            if file_type.is_media_type() {
                tracing::info!("File {} is a recognized media type ({:?})", scan_event.message.object_name, file_type);
                queue_metadata_extraction_completed_event(&scan_event.message.presigned_url, &scan_event.message.object_name, &file_type).await;
                queue_resource_type_update_event(&scan_event.message.object_name, &file_type).await
            } else {
                tracing::warn!("File {} is not a recognized media type, skipping further processing.", scan_event.message.object_name);
                queue_resource_status_update_event(&scan_event.message.object_name, "failed").await;
            }


            delete_message(&client, &queue_url, &scan_event.receipt_handle).await
                .unwrap_or_else(|err| {
                    tracing::error!("Error deleting message: {}", err);
                });
        }


        // Sleep for a while before checking the queue again
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}


async fn receive_virus_scan_completed_notification(client: &Client, queue_url: &str) -> Result<Option<ScanEvent>, aws_sdk_sqs::Error> {
    let rcv_message_output = client
        .receive_message()
        .queue_url(queue_url)
        .max_number_of_messages(1)
        .send()
        .await?;

    
    for message in rcv_message_output.messages.unwrap_or_default() {

        let body = match message.body {
            Some(body) => body,
            None => {
                tracing::warn!("Received message with no body, skipping.");
                continue;
            }
        };

        let scan_message: ScanMessage = match serde_json::from_str(&body){
            Ok(msg) => msg,
            Err(err) => {
                tracing::error!("Failed to parse message body as JSON: {}", err);
                continue;
            }
        };

        return Ok(Some(ScanEvent {
            message: scan_message,
            receipt_handle: message.receipt_handle.unwrap_or_default(),
        }));         

    }

    Ok(None)
}

async fn discover_filetype_and_metadata(presigned_url: &str) -> Result<FileType, io::Error> {
    let media_info = get_file_metadata(presigned_url).await?; 
    // find the General track
    create_metadata_object(&media_info)
}

async fn get_file_metadata(presigned_url: &str) -> Result<MediaInfo, io::Error> {
    // there seem not to be too many maintained libraries for accessing MediaInfo in Rust, so we will use the command line tool
    // and parse the output. Thankfully it provides JSON output.

    let mut child = Command::new("mediainfo")
        .arg("--Output=JSON")
        .arg(presigned_url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MediaInfo process");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let mut stdout_lines = vec![];
    let mut stderr_lines = vec![];

    loop {
        select! {
            line = stdout_reader.next_line() => {
                match line? {
                    Some(line) => stdout_lines.push(line),
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
        tracing::error!("MediaInfo command failed with status: {:?}\nStderr: {}", status, stderr_output);
        return Err(io::Error::new(io::ErrorKind::Other, "MediaInfo command failed"));
    }

    let stdout_output = stdout_lines.join("\n");

    let media_info: MediaInfo = serde_json::from_str(&stdout_output)
        .map_err(|err| {
            tracing::error!("Failed to parse MediaInfo output as JSON: {}", err);
            io::Error::new(io::ErrorKind::InvalidData, "Failed to parse MediaInfo output")
        })?;


    Ok(media_info)
   
}

fn create_metadata_object(media_info: &MediaInfo) -> Result<FileType, io::Error> {
    
    let general_track = media_info.media.track.iter().find(|track| matches!(track, Track::General { .. }));

    let (video_count, audio_count, image_count) = if let Some(
        Track::General { 
            file_size,
            duration,
            format,
            video_count,
            audio_count,
            image_count,other_count }
        ) = general_track {
        
        // isizes in case we for SOME reason get negative values. This would be very unexpected, but who knows if MediaInfo might have bugs, or other funky
        // behaviour. 
        (
            video_count.clone().unwrap_or("0".to_string()).parse::<isize>().unwrap_or(0),
            audio_count.clone().unwrap_or("0".to_string()).parse::<isize>().unwrap_or(0),
            image_count.clone().unwrap_or("0".to_string()).parse::<isize>().unwrap_or(0),
        )
    } else {
        (0isize, 0isize, 0isize)
    };


    if video_count > 0 {
        let video_track = media_info.media.track.iter().find(|track| matches!(track, Track::Video { .. })).unwrap(); 
        // audio track is not guaranteed to exist
        let audio_track = media_info.media.track.iter().find(|track| matches!(track, Track::Audio { .. }));
        Ok(extract_video_information(video_track, audio_track))
    } else if audio_count > 0 {
        // audio
        let audio_track = media_info.media.track.iter().find(|track| matches!(track, Track::Audio { .. })).unwrap();
        Ok(extract_audio_information(audio_track))
    } else if image_count > 0 {
        // image
        let image_track = media_info.media.track.iter().find(|track| matches!(track, Track::Image { .. })).unwrap();
        Ok(extract_image_information(image_track))
    } else {
        Ok(FileType::Other)
    }
}

fn extract_video_information(video_track: &Track, audio_track: Option<&Track>) -> FileType {
    if let Track::Video { width, height, duration, frame_rate, bitrate } = video_track {
        let video_data = VideoData {
            duration: duration.parse().unwrap_or(0.0),
            width: width.parse().unwrap_or(0) as u32,
            height: height.parse().unwrap_or(0) as u32,
            frame_rate: frame_rate.parse().unwrap_or(0.0),
            bitrate: bitrate.parse().unwrap_or(0) as u32,
        };

        let audio_data = audio_track.map(|track| {
            if let Track::Audio { duration, bitrate, sample_rate } = track {
                AudioData {
                    duration: duration.parse().unwrap_or(0.0),
                    bitrate: bitrate.parse().unwrap_or(0) as u32,
                    sample_rate: sample_rate.parse().unwrap_or(0) as u32,
                }
            } else {
                panic!("Expected audio track, but found: {:?}", track)
            }
        });

        FileType::Video { video: video_data, audio: audio_data }
    } else {
        tracing::error!("Expected video track, but found: {:?}", video_track);
        FileType::Other
    }
}

fn extract_audio_information(audio_track: &Track) -> FileType {
    if let Track::Audio { duration, bitrate, sample_rate } = audio_track {
        let audio_data = AudioData {
            duration: duration.parse().unwrap_or(0.0),
            bitrate: bitrate.parse().unwrap_or(0) as u32,
            sample_rate: sample_rate.parse().unwrap_or(0) as u32,
        };
        FileType::Audio { audio: audio_data }
    } else {
        tracing::error!("Expected audio track, but found: {:?}", audio_track);
        FileType::Other
    }
}

fn extract_image_information(image_track: &Track) -> FileType {
    if let Track::Image { width, height } = image_track {
        let image_data = ImageData {
            width: width.parse().unwrap_or(0) as u32,
            height: height.parse().unwrap_or(0) as u32,
        };
        FileType::Image { image: image_data }
    } else {
        tracing::error!("Expected image track, but found: {:?}", image_track);
        FileType::Other
    }
}

async fn queue_metadata_extraction_completed_event(presigned_uri: &str, object_name: &str, file_type: &FileType) {
    let sqs_client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);

    let json_msg = serde_json::json!({
        "presigned_url": presigned_uri,
        "object_name": object_name,
        "file_type": file_type,
    }).to_string(); 

    let queue_url = match file_type {
        FileType::Video { .. } => env::var("VIDEO_PROCESSING_QUEUE_URL").expect("VIDEO_PROCESSING_QUEUE_URL not set"),
        FileType::Audio { .. } => env::var("AUDIO_PROCESSING_QUEUE_URL").expect("AUDIO_PROCESSING_QUEUE_URL not set"),
        FileType::Image { .. } => env::var("IMAGE_PROCESSING_QUEUE_URL").expect("IMAGE_PROCESSING_QUEUE_URL not set"),
        FileType::Other => {
            tracing::warn!("File type is Other, not sending to processing queue.");
            return;
        }
    };
    
    tracing::info!("Sending message {} to SQS queue: {}", json_msg, queue_url);

    sqs_client.send_message()
        .queue_url(queue_url)
        .message_body(json_msg)
        .send()
        .await
        .expect("Failed to send message to SQS");
}

async fn queue_resource_status_update_event(object_name: &str, status: &str) {
    let sqs_client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);
    let queue_url = env::var("RESOURCE_STATUS_QUEUE_URL").expect("RESOURCE_STATUS_QUEUE_URL not set");

    let json_msg = serde_json::json!({
        "object_name": object_name,
        "status": status,
    }).to_string(); 

    tracing::info!("Sending resource status update message {} to SQS queue: {}", json_msg, queue_url);

    sqs_client.send_message()
        .queue_url(queue_url)
        .message_body(json_msg)
        .send()
        .await
        .expect("Failed to send resource status update message to SQS");
}

async fn queue_resource_type_update_event(object_name: &str, metadata: &FileType) {
    let sqs_client = aws_sdk_sqs::Client::new(&aws_config::load_from_env().await);
    let queue_url = env::var("RESOURCE_STATUS_QUEUE_URL").expect("RESOURCE_STATUS_QUEUE_URL not set");

    let json_msg = serde_json::json!({
        "object_name": object_name,
        "status": "type_resolved",
        "resource_type": metadata.as_str(),
    }).to_string(); 

    tracing::info!("Sending resource type update message {} to SQS queue: {}", json_msg, queue_url);

    sqs_client.send_message()
        .queue_url(queue_url)
        .message_body(json_msg)
        .send()
        .await
        .expect("Failed to send resource type update message to SQS");
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