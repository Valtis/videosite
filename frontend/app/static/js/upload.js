"use strict";



async function uploadFile(fileInput) {
    const fileHandle = getFile(fileInput);
    if (!fileHandle) {
        return;
    }

    const  crc = await calculateCrc32(fileHandle);
    const [upload_id, chunk_size] = await initUpload(fileHandle, crc);
    if (!upload_id) {
        console.error("Failed to initiate upload.");
        return;
    }
    console.log(`Upload ID: ${upload_id}, Chunk Size: ${chunk_size}`);
    await uploadFileInChunks(fileHandle, upload_id, chunk_size);
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
        console.error('Failed to initiate upload:', response.statusText);
        return;
    }

    let data = await response.json();
    return [data.upload_id, data.chunk_size];
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

        let response = await fetch(`/upload/chunk?${params}`, {
            method: 'POST',
            credentials: 'include',
            body: formData,
        });

        if (!response.ok) {
            console.error(`Failed to upload chunk ${chunkIndex}:`, response.statusText);
            return;
        }
        console.log(`Uploaded chunk ${chunkIndex + 1} of ${totalChunks}`);
    }

    let response = await fetch ('/upload/complete_chunk_upload', {
        method: 'POST',
        credentials: 'include',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({ upload_id })
    });

    if (!response.ok) {
        console.error('Failed to complete upload:', response.statusText);
        return;
    }
}