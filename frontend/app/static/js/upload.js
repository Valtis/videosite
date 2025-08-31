"use strict";



async function uploadFile(fileInput) {
    const fileHandle = getFile(fileInput);
    if (!fileHandle) {
        return;
    }

    // Show progress container and set filename
    showUploadProgress(fileHandle.name);
    updateUploadProgress(0, 0, 0, 'Calculating checksum...');

    try {
        const crc = await calculateCrc32(fileHandle);
        const [upload_id, chunk_size] = await initUpload(fileHandle, crc);
        if (!upload_id) {
            // Error already handled and displayed by initUpload
            return;
        }
        
        // Update progress to show we're starting the upload
        updateUploadProgress(0, 0, Math.ceil(fileHandle.size / chunk_size), 'Starting upload...');
        
        console.log(`Upload ID: ${upload_id}, Chunk Size: ${chunk_size}`);
        await uploadFileInChunks(fileHandle, upload_id, chunk_size);
    } catch (error) {
        console.error('Error during file upload:', error);
        showUploadError('Failed to read file. Please try again');
    }
}


function getFile(fileInput) {

    if (!fileInput || fileInput.files.length === 0) {
        return null;
    }

    return fileInput.files[0];
}


async function calculateCrc32(file) {
    const table = (function makeTable() {
        const t = new Uint32Array(256);
        for (let n = 0; n < 256; n++) {
            let c = n;
            for (let k = 0; k < 8; k++) {
                c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
            }
            t[n] = c >>> 0;
        }
        return t;
    })();

    let crc = 0xFFFFFFFF >>> 0;
    const reader = file.stream().getReader();

    while (true) {
        const { done, value } = await reader.read();
        if (done) {
            break;
        }

        const chunk = value instanceof Uint8Array ? value : new Uint8Array(value);
        for (let i = 0; i < chunk.length; i++) {
            crc = (crc >>> 8) ^ table[(crc ^ chunk[i]) & 0xFF];
        }
    }

    crc = (crc ^ 0xFFFFFFFF) >>> 0;
    // return as 8-char hex
    return crc.toString(16).padStart(8, '0');
}

async function initUpload(fileHandle, crc) {
    try {
        let response = await fetch('/upload/init_chunk_upload', {
            method: 'POST',
            credentials: 'include',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ 
                file_name: fileHandle.name,
                file_size: fileHandle.size,
                integrity_check_type: 'crc32',
                integrity_check_value: crc
            })
        });
        
        if (!response.ok) {
            const errorMessage = await handleResponseError(response, 'Upload initialization');
            showUploadError(errorMessage);
            return [null, null];
        }

        let data = await response.json();
        return [data.upload_id, data.chunk_size];
    } catch (error) {
        console.error('Network error during upload initialization:', error);
        showUploadError('Network error. Please check your connection');
        return [null, null];
    }
}

async function uploadFileInChunks(fileHandle, upload_id, chunk_size) {
    const totalChunks = Math.ceil(fileHandle.size / chunk_size);
    
    for (let chunkIndex = 0; chunkIndex < totalChunks; chunkIndex++) {
        const start = chunkIndex * chunk_size;
        const end = Math.min(start + chunk_size, fileHandle.size);
        
        const chunk = fileHandle.slice(start, end);

        const formData = new FormData();
        formData.append('file', chunk);

        const params = new URLSearchParams({
            upload_id,
            chunk_index: chunkIndex + 1 // 1-based index
        });

        try {
            let response = await fetch(`/upload/chunk?${params}`, {
                method: 'POST',
                credentials: 'include',
                body: formData,
            });

            if (!response.ok) {
                const errorMessage = await handleResponseError(response, `Chunk ${chunkIndex + 1} upload`);
                showUploadError(errorMessage);
                return;
            }
            
            // Update progress
            const progress = ((chunkIndex + 1) / totalChunks) * 100;
            updateUploadProgress(progress, chunkIndex + 1, totalChunks);
            
            console.log(`Uploaded chunk ${chunkIndex + 1} of ${totalChunks}`);
        } catch (error) {
            console.error(`Network error uploading chunk ${chunkIndex + 1}:`, error);
            showUploadError('Network error during upload. Please check your connection');
            return;
        }
    }

    // Complete the upload
    try {
        let response = await fetch('/upload/complete_chunk_upload', {
            method: 'POST',
            credentials: 'include',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({ upload_id })
        });

        if (!response.ok) {
            const errorMessage = await handleResponseError(response, 'Upload completion');
            showUploadError(errorMessage);
            return;
        }
        
        // Upload completed successfully
        completeUploadProgress();
        
        // Update quota information after successful upload
        if (typeof initQuota === 'function') {
            initQuota();
        }
        
        // Also refresh the resources list to show the new upload
        if (typeof loadResources === 'function') {
            loadResources();
        }
    } catch (error) {
        console.error('Network error completing upload:', error);
        showUploadError('Network error during upload completion. Please check your connection');
        return;
    }
}

// Progress bar control functions
function showUploadProgress(filename) {
    const progressContainer = document.getElementById('upload-progress-container');
    const filenameElement = document.getElementById('upload-filename');
    const progressFill = document.getElementById('upload-progress-fill');
    const progressText = document.getElementById('upload-progress-text');
    
    filenameElement.textContent = filename;
    progressFill.style.width = '0%';
    progressText.textContent = '0%';
    progressContainer.classList.remove('hidden', 'fade-out');
}

function updateUploadProgress(percentage, currentChunk, totalChunks, statusMessage = null) {
    const progressFill = document.getElementById('upload-progress-fill');
    const progressText = document.getElementById('upload-progress-text');
    
    if (statusMessage) {
        progressText.textContent = statusMessage;
        progressFill.style.width = '0%';
    } else {
        const roundedPercentage = Math.round(percentage);
        progressFill.style.width = `${percentage}%`;
        progressText.textContent = `${roundedPercentage}% (${currentChunk}/${totalChunks})`;
    }
}

function completeUploadProgress() {
    const progressText = document.getElementById('upload-progress-text');
    progressText.textContent = '100% - Complete!';
    
    // Fade out after 2 seconds
    setTimeout(() => {
        const progressContainer = document.getElementById('upload-progress-container');
        progressContainer.classList.add('fade-out');
        
        // Hide completely after fade animation
        setTimeout(() => {
            progressContainer.classList.add('hidden');
            // Reset file input
            document.getElementById('fileInput').value = '';
        }, 500);
    }, 2000);
}

function hideUploadProgress() {
    const progressContainer = document.getElementById('upload-progress-container');
    progressContainer.classList.add('hidden');
    progressContainer.classList.remove('fade-out');
    // Reset file input
    document.getElementById('fileInput').value = '';
}

// Error handling functions
function showUploadError(message) {
    const progressText = document.getElementById('upload-progress-text');
    const progressFill = document.getElementById('upload-progress-fill');
    const progressContainer = document.getElementById('upload-progress-container');
    
    progressText.textContent = `Error: ${message}`;
    progressText.classList.add('error');
    progressFill.style.width = '0%';
    progressFill.classList.add('error');
    
    // Show error for 5 seconds, then hide
    setTimeout(() => {
        progressContainer.classList.add('fade-out');
        setTimeout(() => {
            progressContainer.classList.add('hidden');
            progressText.classList.remove('error');
            progressFill.classList.remove('error');
            // Reset file input
            document.getElementById('fileInput').value = '';
        }, 500);
    }, 5000);
}

async function handleResponseError(response, operation) {
    let errorMessage = `${operation} failed`;
    
    try {
        // Try to get error details from response body
        const errorData = await response.json();
        if (errorData.error) {
            errorMessage = errorData.error;
            if (errorMessaged === 'quota_exceeded') {
                errorMessage = 'Storage quota exceeded';
            }
        } else if (errorData.message) {
            errorMessage = errorData.message;
        }
    } catch (e) {
        // If we can't parse JSON, use status-based messages
        if (response.status === 402) {
            errorMessage = 'Storage quota exhausted';
        } else if (response.status === 413) {
            errorMessage = 'File too large';
        } else if (response.status === 429) {
            errorMessage = 'Rate limit exceeded. Please try again later';
        } else if (response.status === 507) {
            errorMessage = 'Storage quota exceeded';
        } else if (response.status >= 400 && response.status < 500) {
            errorMessage = 'Invalid request. Please check your file and try again';
        } else if (response.status >= 500) {
            errorMessage = 'Server error. Please try again later';
        }
    }
    
    return errorMessage;
}