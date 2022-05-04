#!/usr/bin/env bash

set -o errexit
set -o nounset

pip3 install -r requirements.txt

DOWNLOAD_URL=$(curl --silent 'https://api.github.com/repos/alexwlchan/dominant_colours/releases/latest' \
  | jq -r ' .assets | map(.browser_download_url) | map(select(test(".*linux.*")))[0]'
)

# The --location flag means we follow redirects
curl --location "$DOWNLOAD_URL" > dominant_colours.tar.gz
tar -xzf dominant_colours.tar.gz

mv dominant_colours
chmod +x dominant_colours
./dominant_colours --version

if [[ "$DEBUG" == "yes" ]]
then
  python3 server.py
else
  gunicorn server:app -w 4 --log-file -
fi