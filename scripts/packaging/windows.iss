#ifndef Payload
  #error Payload must point to the gated Windows staging directory
#endif
#ifndef Version
  #error Version must come from Cargo.toml
#endif
#ifndef Output
  #error Output directory required
#endif
[Setup]
AppId={{46065CA8-830A-499A-9D8E-BC47DFDFDF16}
AppName=LibreToon
AppVersion={#Version}
AppPublisher=alvillafm
AppPublisherURL=https://github.com/alvillafm/LibreToon
VersionInfoCompany=alvillafm
VersionInfoDescription=LibreToon installer
VersionInfoProductName=LibreToon
VersionInfoProductVersion={#Version}
VersionInfoVersion={#Version}
#ifdef SignBuild
SignTool=LibreToon
SignedUninstaller=yes
SignedUninstallerDir={#SignCache}
#endif
DefaultDirName={localappdata}\Programs\LibreToon
DefaultGroupName=LibreToon
PrivilegesRequired=lowest
UsePreviousAppDir=yes
UsePreviousGroup=yes
Uninstallable=yes
CreateUninstallRegKey=yes
UninstallDisplayName=LibreToon
CloseApplications=no
RestartApplications=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputDir={#Output}
OutputBaseFilename=LibreToon-{#Version}-Windows-x64-Setup
SetupIconFile=..\..\assets\libretoon.ico
UninstallDisplayIcon={app}\libretoon.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile={#Payload}\LICENSE
[Files]
Source: "{#Payload}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{group}\Uninstall LibreToon"; Filename: "{uninstallexe}"
Name: "{group}\LibreToon"; Filename: "{app}\libretoon.exe"
[Run]
Filename: "{app}\libretoon.exe"; Description: "Open LibreToon"; Flags: nowait postinstall skipifsilent unchecked
