; Programme d'installation Windows de Glyphfall.
;
; Compilé par Inno Setup, et non par NSIS : celui-ci affiche « Nullsoft Install
; System » au bas de chaque page, et l'effacer demande de bricoler une
; directive. Inno Setup n'appose aucune marque, et sa traduction française est
; livrée avec l'outil.
;
;   ISCC.exe /DVERSION=0.2.1 installer\glyphfall.iss
;
; La version est passée par la ligne de commande, comme partout ailleurs dans
; ce dépôt : elle vient du tag de la release.

#ifndef VERSION
  #define VERSION "0.0.0"
#endif

; Windows n'accepte qu'une suite de nombres dans les informations de version
; du fichier. Un tag de pre-publication comme « 0.3.0-beta » y ferait echouer
; la compilation, alors qu'il s'affiche tres bien partout ailleurs.
#define VERSION_NUM Copy(VERSION, 1, Pos("-", VERSION) > 0 ? Pos("-", VERSION) - 1 : Len(VERSION))

#define NOM "Glyphfall"
#define EDITEUR "Game K"
#define EXECUTABLE "glyphfall.exe"

[Setup]
AppId={{7A2F1C64-9E1B-4C3D-9A5E-6B0D2F8E4A11}
AppName={#NOM}
AppVersion={#VERSION}
AppVerName={#NOM} {#VERSION}
AppPublisher={#EDITEUR}
VersionInfoVersion={#VERSION_NUM}

; Le joueur choisit où installer. Le dossier proposé est celui des programmes,
; qui se résout dans le profil de l'utilisateur puisqu'on n'exige pas
; l'élévation.
DefaultDirName={autopf}\{#NOM}
DefaultGroupName={#NOM}
AllowNoIcons=yes
DisableProgramGroupPage=yes

; Pas d'invite d'élévation : le jeu n'écrit rien hors de son dossier et de la
; sauvegarde du joueur. Demander les droits administrateur ajouterait un
; avertissement de plus à celui de SmartScreen, sans rien apporter.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

OutputDir=.
OutputBaseFilename=glyphfall-windows-x86_64-setup
SetupIconFile=glyphfall.ico
UninstallDisplayIcon={app}\{#EXECUTABLE}
UninstallDisplayName={#NOM} {#VERSION}

; L'exécutable embarque déjà musiques, voix et polices : il ne se comprime
; presque plus. LZMA2 au maximum ne coûte que du temps de construction.
Compression=lzma2/max
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

WizardStyle=modern
LicenseFile=..\LICENSE

[Languages]
Name: "francais"; MessagesFile: "compiler:Languages\French.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "raccourcibureau"; Description: "{cm:CreateDesktopIcon}"; \
    GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#EXECUTABLE}"; DestDir: "{app}"; Flags: ignoreversion
Source: "glyphfall.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\fonts\LICENCES.md"; DestDir: "{app}"; \
    DestName: "LICENCES-POLICES.md"; Flags: ignoreversion

[Icons]
; L'icône est prise dans le fichier livré à côté : l'exécutable n'embarque pas
; encore de ressource d'icône, et un raccourci sans icône fait négligé.
Name: "{group}\{#NOM}"; Filename: "{app}\{#EXECUTABLE}"; \
    IconFilename: "{app}\glyphfall.ico"
Name: "{group}\{cm:UninstallProgram,{#NOM}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#NOM}"; Filename: "{app}\{#EXECUTABLE}"; \
    IconFilename: "{app}\glyphfall.ico"; Tasks: raccourcibureau

[Run]
Filename: "{app}\{#EXECUTABLE}"; Description: "{cm:LaunchProgram,{#NOM}}"; \
    Flags: nowait postinstall skipifsilent
