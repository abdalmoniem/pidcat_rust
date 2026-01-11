#define AppName "PidCat"
#define AppVersion "1.0.0"
#define AppPublisher "AbdElMoniem ElHifnawy"
#define AppURL "https://abdalmoniem-alhifnawy.is-a.dev"
#define AppExeName "PidCat.exe"
#define DateTime GetDateTimeString('ddd_dd_mmm_yyyy_hh_nn_ss.zzz_ampm', '', '')
#define SetupDir ExtractFilePath(SourcePath)
#define BuildDir ExtractFilePath(RemoveBackslashUnlessRoot(SetupDir))
#define ProjectDir ExtractFilePath(RemoveBackslashUnlessRoot(BuildDir))

[Setup]
AppId={{FA8A4F6A-6C74-4544-8B54-1481B07F996C}

AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={code:GetDefaultDirName}
UninstallDisplayIcon={app}\{#AppExeName}
; "ArchitecturesAllowed=x64compatible" specifies that Setup cannot run
; on anything but x64 and Windows 11 on Arm.
ArchitecturesAllowed=x64compatible
; "ArchitecturesInstallIn64BitMode=x64compatible" requests that the
; install be done in "64-bit mode" on x64 or Windows 11 on Arm,
; meaning it should use the native 64-bit Program Files directory and
; the 64-bit view of the registry.
ArchitecturesInstallIn64BitMode=x64compatible
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
InfoBeforeFile=info.txt
ChangesEnvironment=true
; Uncomment the following line to run in non administrative install mode (install for current user only).
PrivilegesRequired=lowest
; PrivilegesRequiredOverridesAllowed=dialog
OutputBaseFilename={#AppName}_v{#AppVersion}_{#DateTime}
SetupIconFile={#ProjectDir}\assets\icon.ico
SolidCompression=yes
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "{#ProjectDir}\target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent unchecked

; Use ping.exe as a dummy, quick-running command to trigger the AfterInstall function
Filename: "{sys}\ping.exe"; Description: "Add program directory to the system PATH"; Flags: nowait skipifsilent postinstall runhidden; AfterInstall: EnvAddPath(ExpandConstant('{app}'))

[Code]
var
  InstallOptionsPage: TWizardPage;
  ReInstallRadioButton: TNewRadioButton;
  UninstallRadioButton: TNewRadioButton;
  IsAlreadyInstalled: Boolean;

const EnvironmentKey = 'Environment';

procedure ExitProcess(uExitCode: Integer);
  external 'ExitProcess@kernel32.dll stdcall';

function GetDefaultDirName(Param: String): String;
begin
  if IsAdmin then
    Result := ExpandConstant('{commonpf}\{#AppPublisher}\{#AppName}')
  else
    Result := ExpandConstant('{localappdata}\Programs\{#AppPublisher}\{#AppName}');
end;

procedure EnvAddPath(Path: string);
var
    Paths: string;
begin
    { Retrieve current path (use empty string if entry not exists) }
    if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths)
    then Paths := '';

    { Skip if string already found in path }
    if Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';') > 0 then exit;

    { App string to the end of the path variable }
    Paths := Paths + ';'+ Path +';'

    { Overwrite (or create if missing) path environment variable }
    if RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths)
    then Log(Format('The [%s] added to PATH: [%s]', [Path, Paths]))
    else Log(Format('Error while adding the [%s] to PATH: [%s]', [Path, Paths]));
end;

procedure EnvRemovePath(Path: string);
var
    Paths: string;
    P: Integer;
begin
    { Skip if registry entry not exists }
    if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
        exit;

    { Skip if string not found in path }
    P := Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';');
    if P = 0 then exit;

    { Update path variable }
    Delete(Paths, P - 1, Length(Path) + 1);

    { Overwrite path environment variable }
    if RegWriteStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths)
    then Log(Format('The [%s] removed from PATH: [%s]', [Path, Paths]))
    else Log(Format('Error while removing the [%s] from PATH: [%s]', [Path, Paths]));
end;

function GetUninstallString(const AppId: string): string;
var
  S: string;
begin
  Result := '';
  if RegQueryStringValue(HKLM, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\' + AppId + '_is1', 'UninstallString', S) then
    Result := S
  else if RegQueryStringValue(HKCU, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\' + AppId + '_is1', 'UninstallString', S) then
    Result := S;
end;

// This procedure creates the custom page with our custom buttons.
procedure CreateOptionsPage;
var
  ReInstallDescLabel: TLabel;
  UninstallDescLabel: TLabel;
begin
  InstallOptionsPage := CreateCustomPage(
    wpWelcome,
    'Installation Options',
    'Choose how to proceed with the installation'
  );
  
  ReInstallRadioButton := TNewRadioButton.Create(InstallOptionsPage);
  with ReInstallRadioButton do begin
    Parent := InstallOptionsPage.Surface;
    Checked := True;
    Top := 15;
    Width := InstallOptionsPage.SurfaceWidth;
    Font.Style := [fsBold];
    Font.Size := 10;
    Caption := 'Re-Install'
  end;
    
  ReInstallDescLabel := TLabel.Create(InstallOptionsPage);
  with ReInstallDescLabel do begin
    Parent := InstallOptionsPage.Surface;
    Left := 5;
    Top := ReInstallRadioButton.Top + ReInstallRadioButton.Height + 8;
    Width := InstallOptionsPage.SurfaceWidth; 
    Height := 40;
    AutoSize := False;
    Wordwrap := True;
    Caption := 'Re-Install. Will reinstall the application again with new settings';
  end;
  
  UninstallRadioButton := TNewRadioButton.Create(InstallOptionsPage);
  with UninstallRadioButton do begin
    Checked := False;
    Parent := InstallOptionsPage.Surface;
    Top := ReInstallDescLabel.Top + ReInstallDescLabel.Height + 16;
    Width := InstallOptionsPage.SurfaceWidth;
    Font.Style := [fsBold];
    Font.Size := 10;
    Caption := 'Uninstall'
  end;
  
  UninstallDescLabel := TLabel.Create(WizardForm);
  with UninstallDescLabel do begin
    Parent := InstallOptionsPage.Surface;
    Left := 5;
    Top := UninstallRadioButton.Top + UninstallRadioButton.Height + 8;
    Width := InstallOptionsPage.SurfaceWidth;
    Height := 40;
    AutoSize := False;
    Wordwrap := True;
    Caption := 'Uninstall. Removes the application from your computer';
  end;
end;

// This function runs at the start of the setup. It should not access wizard pages.
function InitializeSetup(): Boolean;
var
  UninstallPath: string;
begin
  Result := True;
  IsAlreadyInstalled := False;

  // Check for an existing installation using the AppId.
  UninstallPath := GetUninstallString(ExpandConstant('{#emit SetupSetting("AppId")}'));
  
  if UninstallPath <> '' then begin
    IsAlreadyInstalled := True;
  end;
end;

// InitializeWizard is the correct place to create custom wizard pages.
procedure InitializeWizard;
var
  Suffix: String;
begin
  if IsAdmin then
    Suffix := '⁂ Admin'
  else
    Suffix := '⌂ ' + GetUserNameString();
  
  WizardForm.Caption := Format('Setup - {#AppName} v{#AppVersion} {#DateTime} (%s)', [Suffix]);
  
  if IsAlreadyInstalled then begin
    CreateOptionsPage;
  end;
end;

// This function controls the wizard's behavior based on user choices.
function NextButtonClick(CurPageID: Integer): Boolean;
var
  UninstallPath: string;
  ResultCode: Integer;
begin
  Result := True;
  
  // If the user is on our custom page...
  if (InstallOptionsPage <> nil) and (CurPageID = InstallOptionsPage.ID) then
  begin
    if ReInstallRadioButton.Checked then begin
      // Re-Install: Continue with the installation.
    end else if UninstallRadioButton.Checked then begin
      // Uninstall: Launch the uninstaller and abort the current setup.
      UninstallPath := RemoveQuotes(GetUninstallString(ExpandConstant('{#emit SetupSetting("AppId")}')));
      if not Exec(UninstallPath, '', '', SW_SHOW, ewWaitUntilTerminated, ResultCode) then
      begin
        ExitProcess(ResultCode);
      end
      else
      begin
        ExitProcess(0);
      end;
    end;
  end;
  
  Result := True;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then if WizardSilent then EnvAddPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
    if CurUninstallStep = usPostUninstall then EnvRemovePath(ExpandConstant('{app}'));
end;
