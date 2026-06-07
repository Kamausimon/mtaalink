# MtaaLink development seed script
# Run with: .\scripts\seed.ps1
# Prerequisites: backend on http://localhost:7878, psql in PATH
# All test accounts use password: test1234

param(
    [string]$ApiUrl = "http://localhost:7878",
    [string]$DbUrl  = "postgres://postgres:Kamau0746@localhost:5432/postgres",
    [string]$Psql   = "C:\Program Files\PostgreSQL\17\bin\psql.exe"
)

$ErrorActionPreference = "Stop"

function Register-User {
    param([hashtable]$body)
    $json = $body | ConvertTo-Json -Compress
    try {
        $res = Invoke-RestMethod -Method POST -Uri "$ApiUrl/auth/register" `
            -ContentType "application/json" -Body $json
        Write-Host "  OK   $($body['username']) (user_id=$($res.user_id))" -ForegroundColor Green
        return [int]$res.user_id
    } catch {
        $email = $body['email']
        $uname = $body['username']
        Write-Host "  SKIP $uname - already registered, looking up ID" -ForegroundColor DarkYellow
        $rows = & $Psql $DbUrl -t -A -c "SELECT id FROM users WHERE username='$uname' OR email='$email' LIMIT 1;" 2>&1
        $id = $rows | Where-Object { $_ -match '^\d+$' } | Select-Object -First 1
        if (-not $id) { Write-Error "Could not find user ID for $uname / $email"; return }
        Write-Host "       found user_id=$id" -ForegroundColor DarkYellow
        return [int]$id
    }
}

Write-Host ""
Write-Host "=== MtaaLink Seed Script ===" -ForegroundColor Cyan

# Step 1: Categories
Write-Host ""
Write-Host "[1/4] Seeding categories..." -ForegroundColor Cyan
$catScript = Join-Path $PSScriptRoot "seed_categories.sql"
& $Psql $DbUrl -f $catScript

# Step 2: Register users via API
Write-Host ""
Write-Host "[2/4] Registering users..." -ForegroundColor Cyan

$c1id = Register-User @{ username="janewanjiku";  email="jane@mtaatest.com";           password="test1234"; confirm_password="test1234"; role="client" }
$c2id = Register-User @{ username="davidmwangi";  email="david@mtaatest.com";          password="test1234"; confirm_password="test1234"; role="client" }

$p1id = Register-User @{ username="johnkamau";    email="john.plumber@mtaatest.com";   password="test1234"; confirm_password="test1234"; role="provider"; service_description="Expert plumber with 10 years experience in residential and commercial plumbing across Nairobi" }
$p2id = Register-User @{ username="marynjoki";    email="mary.hair@mtaatest.com";      password="test1234"; confirm_password="test1234"; role="provider"; service_description="Professional hair stylist specialising in braids, weaves, and natural hair care" }
$p3id = Register-User @{ username="peterotieno";  email="peter.elec@mtaatest.com";     password="test1234"; confirm_password="test1234"; role="provider"; service_description="Certified electrician for domestic and commercial installations and repairs" }
$p4id = Register-User @{ username="gracemuthoni"; email="grace.clean@mtaatest.com";    password="test1234"; confirm_password="test1234"; role="provider"; service_description="Reliable and thorough house cleaner available weekdays and weekends" }
$p5id = Register-User @{ username="samuelachieng"; email="samuel.tutor@mtaatest.com"; password="test1234"; confirm_password="test1234"; role="provider"; service_description="KCSE maths and physics tutor with a track record of improving student grades" }

$b1id = Register-User @{ username="cleanprobiz";  email="cleanpro@mtaatest.com";       password="test1234"; confirm_password="test1234"; role="business"; business_name="CleanPro Services" }
$b2id = Register-User @{ username="techfixkenya"; email="techfix@mtaatest.com";        password="test1234"; confirm_password="test1234"; role="business"; business_name="TechFix Kenya" }

# Step 3: Seed profiles, services, reviews, bookings via SQL
Write-Host ""
Write-Host "[3/4] Seeding profiles, services, reviews and bookings..." -ForegroundColor Cyan

$dataScript = Join-Path $PSScriptRoot "seed_data.sql"
& $Psql $DbUrl `
    -v c1id=$c1id -v c2id=$c2id `
    -v p1id=$p1id -v p2id=$p2id -v p3id=$p3id -v p4id=$p4id -v p5id=$p5id `
    -v b1id=$b1id -v b2id=$b2id `
    -f $dataScript

# Step 4: Summary
Write-Host ""
Write-Host "[4/4] Done!" -ForegroundColor Cyan
Write-Host ""
Write-Host "Test accounts (password: test1234):" -ForegroundColor White
Write-Host "  Clients:   jane@mtaatest.com  /  david@mtaatest.com"
Write-Host "  Providers: john.plumber@mtaatest.com  mary.hair@mtaatest.com"
Write-Host "             peter.elec@mtaatest.com    grace.clean@mtaatest.com"
Write-Host "             samuel.tutor@mtaatest.com"
Write-Host "  Business:  cleanpro@mtaatest.com  /  techfix@mtaatest.com"
Write-Host ""
