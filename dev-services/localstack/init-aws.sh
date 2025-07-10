#!/bin/bash
awslocal s3 mb s3://upload/
awslocal sqs create-queue --queue-name upload-queue
awslocal sqs create-queue --queue-name type-detection-queue
awslocal sqs create-queue --queue-name transcoding-queue
