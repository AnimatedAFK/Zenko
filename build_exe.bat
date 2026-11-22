@echo off
cd /d "%~dp0"
echo === Building Zenko.exe (one time, needs internet) ===
where python >nul 2>nul || (echo Python not found. Install it from python.org first. & pause & exit /b)
if not exist zenko.py (echo zenko.py is not in this folder. Put all the Zenko files together. & pause & exit /b)
python zenko.py --make-icon zenko.ico
python -m pip install --upgrade pyinstaller rmscene pypdf
echo.
echo === Optional helper for better web search (ddgs) ===
echo If this next step fails or is blocked, that's OK: Zenko still searches the web without it.
python -m pip install --upgrade ddgs
set EXTRA=
python -c "import ddgs" >nul 2>nul && set EXTRA=--hidden-import ddgs --collect-submodules ddgs.engines --collect-submodules primp
python -m PyInstaller --noconfirm --clean --onefile --windowed --name Zenko --icon zenko.ico --add-data "zenko.ico;." --collect-submodules rmscene --collect-submodules pypdf %EXTRA% zenko.py
if exist dist\Zenko.exe (
  copy /y dist\Zenko.exe "%~dp0Zenko.exe" >nul
  copy /y dist\Zenko.exe "%~dp0ZenkoSetup.exe" >nul
  if not exist release mkdir release
  copy /y dist\Zenko.exe release\Zenko.exe >nul
  copy /y zenko.py release\zenko.py >nul
  echo.
  echo Done! Double-click ZenkoSetup.exe to install Zenko for your user (no admin password).
  echo Zenko.exe is the same program if you just want to run it without installing.
  echo For updates: upload the two files in the "release" folder to a new GitHub release.
) else (
  echo Build failed - copy the messages above and send them to Claude.
)
pause
