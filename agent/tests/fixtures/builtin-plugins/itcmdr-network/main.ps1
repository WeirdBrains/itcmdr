# Test fixture only, never shipped: mock of the built-in plugin itcmdr-network (see ../MockBuiltinPlugin.ps1).
. (Join-Path (Split-Path -Parent $PSScriptRoot) 'MockBuiltinPlugin.ps1')
Invoke-ItcmdrMockBuiltinPlugin -Name 'itcmdr-network'
