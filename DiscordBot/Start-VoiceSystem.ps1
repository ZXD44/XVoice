$host.UI.RawUI.WindowTitle = "ZirconX Voice Bot - Auto Restart System"
Set-Location $PSScriptRoot

while ($true) {
    Write-Host "============================" -ForegroundColor Cyan
    Write-Host "   ZIRCON VOICE BOT SYSTEM  " -ForegroundColor Cyan
    Write-Host "============================" -ForegroundColor Cyan
    
    Write-Host ">>> Starting Bot..." -ForegroundColor Green
    
    # Kill existing process on port 3000
    Write-Host ">>> Cleaning up port 3000..." -ForegroundColor Yellow
    $connections = Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue
    if ($connections) {
        foreach ($conn in $connections) {
            Stop-Process -Id $conn.OwningProcess -Force -ErrorAction SilentlyContinue
        }
    }
    Start-Sleep -Seconds 1

    # Run Bot
    npm start
    
    Write-Host "!!! System Closed! Restarting in 3 seconds..." -ForegroundColor Yellow
    Start-Sleep -Seconds 3
    Clear-Host
}
