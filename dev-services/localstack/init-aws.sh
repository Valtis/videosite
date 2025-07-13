#!/bin/bash
awslocal s3 mb s3://upload/
awslocal s3 mb s3://resource/


awslocal sqs create-queue --queue-name upload-finished-queue
awslocal sqs create-queue --queue-name virus-scan-clear-queue
awslocal sqs create-queue --queue-name video-processing-queue
awslocal sqs create-queue --queue-name audio-processing-queue
awslocal sqs create-queue --queue-name image-processing-queue
awslocal sqs create-queue --queue-name resource-status-queue