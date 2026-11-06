// unzip.js <zip> <folder> <files expected>: unpacks a .zip with Windows XP's own Compressed Folders
// (Shell.Application), then waits until the folder holds the expected number of files (the copy runs
// in the background) and prints "#unzipped <n>". Lane WINXP test helper; installs nothing.
// cscript //nologo unzip.js C:\xp\R.zip C:\R 230
var args = WScript.Arguments;
var fso = new ActiveXObject("Scripting.FileSystemObject");
var shell = new ActiveXObject("Shell.Application");
var zip = fso.GetAbsolutePathName(args(0)), dest = fso.GetAbsolutePathName(args(1));
var want = parseInt(args(2), 10);
// (FileSystemObject.CreateFolder makes one level only: the missing parents first)
function makeFolder(path) {
    if (fso.FolderExists(path)) return;
    var parent = fso.GetParentFolderName(path);
    if (parent && !fso.FolderExists(parent)) makeFolder(parent);
    fso.CreateFolder(path);
}
makeFolder(dest);
// 4 no progress, 16 yes to all, 512 no "create folder?", 1024 no error dialog
shell.NameSpace(dest).CopyHere(shell.NameSpace(zip).Items(), 4 | 16 | 512 | 1024);
function count(path) {
    var f = fso.GetFolder(path), n = f.Files.Count, e = new Enumerator(f.SubFolders);
    for (; !e.atEnd(); e.moveNext()) n += count(e.item().Path);
    return n;
}
for (var i = 0; i < 1800; i++) { // at most 30 minutes
    if (count(dest) >= want) break;
    WScript.Sleep(1000);
}
WScript.Sleep(2000); // the last file's bytes
WScript.Echo("#unzipped " + count(dest) + " of " + want);
