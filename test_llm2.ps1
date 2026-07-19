param(
    [string]$ApiBase = "https://api.sfkey.cn/v1",
    [string]$ApiKey = "sk-o4KSSPt7QqGI18ozxCRoy5Jc9wRAKmKlv4kqpd0sKUH7tafJ",
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
Write-Host "API Key Length: $($ApiKey.Length)"

try {
    $response = Invoke-WebRequest -Uri "$ApiBase/chat/completions" -Method POST -Headers $headers -Body $body -TimeoutSec 60
    Write-Host "Status: $($response.StatusCode)"
    $content = $response.Content | ConvertFrom-Json
    Write-Host "Response content: $($content.choices[0].message.content)"
} catch {
    Write-Host "Error: $($_.Exception.Message)"
    if ($_.Exception.Response) {
        Write-Host "Status Code: $($_.Exception.Response.StatusCode.value__)"
    }
}
