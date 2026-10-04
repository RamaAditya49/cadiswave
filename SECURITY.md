# Security

Report security defects through GitHub private vulnerability reporting when available.
Do not publish credentials, private paths, raw diagnostics, or device serials in public issues.

CadisWave checks installation ownership, file ancestry, process ownership, and vendor leases.
Keep configuration directories owned by your login user.
Do not run the desktop application as root.

Run `cadiswave-diag` before submitting an audio issue.
Read its output before sharing it.
Remove user names, application names, device serials, and private paths.

Hardware controls use reverse-engineered protocol mappings.
Unavailable controls must reject commands.
Report the USB ID, application revision, and confirmed observed behavior.
