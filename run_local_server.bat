where /q http-server

IF ERRORLEVEL 1 (npm i -g http-server)

http-server -c-1 -p 80