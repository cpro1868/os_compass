param(
    [string]$ApiBase = "https://api.sfkey.cn/v1",
    [string]$ApiKey = "QCVHf++tA2wMU8nKlLwDUo+CLunZTpjKaGj44J2Co7Yol7kvJBAn5rspr9YYY5oXr9JHWwItWMu86HFotMK/CRME3/dzRDrsvbWSjY5cVA==",
    [string]$Model = "MiniMax-M3"
)

$headers = @{
    "Authorization" = "Bearer $ApiKey"
    "Content-Type" = "application/json"
}

$body = @{
    model = $Model
    messages = @(
        @{
            role = "user"
            content = "Hi"
        }
    )
} | ConvertTo-Json -Compress

Write-Host "Testing LLM API call..."
Write-Host "API Base: $ApiBase"
Write-Host "Model: $Model"

try {
    $response = Invoke-WebRequest -Uri "$ApiBase/chat/completions" -Method POST -Headers $headers -Body $body -TimeoutSec 60
    Write-Host "Status: $($response.StatusCode)"
    Write-Host "Response: $($response.Content | ConvertFrom-Json | ConvertTo-Json -Compress)"
} catch {
    Write-Host "Error: $($_.Exception.Message)"
}
