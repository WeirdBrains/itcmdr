Set-StrictMode -Version Latest

Describe 'agent scripts' {
  It '<_> parses without syntax errors' -ForEach @(Get-ChildItem -Path (Join-Path $PSScriptRoot '..') -Filter '*.ps1' -File | ForEach-Object Name) {
    $path = Join-Path $PSScriptRoot '..' $_
    $tokens = $null; $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors) | Out-Null
    $errors | Should -BeNullOrEmpty
  }

  It 'loads functions in import-only mode without a site config' {
    $env:ITCMDR_IMPORT_ONLY = '1'
    try {
      . (Join-Path $PSScriptRoot '..' 'Invoke-ItcmdrAgent.ps1')
      Get-Command Invoke-ItcmdrCheck -ErrorAction Stop | Should -Not -BeNullOrEmpty
    } finally {
      Remove-Item Env:ITCMDR_IMPORT_ONLY -ErrorAction SilentlyContinue
    }
  }
}
