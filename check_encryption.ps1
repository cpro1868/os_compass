$apiKey = "QCVHf++tA2wMU8nKlLwDUo+CLunZTpjKaGj44J2Co7Yol7kvJBAn5rspr9YYY5oXr9JHWwItWMu86HFotMK/CRME3/dzRDrsvbWSjY5cVA=="
Write-Host "Length: $($apiKey.Length)"

try {
    $decoded = [System.Convert]::FromBase64String($apiKey)
    Write-Host "Base64 decode length: $($decoded.Length)"
    Write-Host "First 20 bytes (hex):"
    for ($i = 0; $i -lt [Math]::Min(20, $decoded.Length); $i++) {
        Write-Host -NoNewline "$([String]::Format('{0:X2}', $decoded[$i])) "
    }
    Write-Host ""
    
    if ($decoded.Length -ge 28) {
        Write-Host "Possible AES-GCM format"
    } else {
        Write-Host "May NOT be AES-GCM format (length < 28)"
    }
} catch {
    Write-Host "Invalid base64: $_"
}
