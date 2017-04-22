#!/bin/bash
DST="${HOME}/.connectr.ini"

echo ""
echo "**************************************************"
echo "**************************************************"
echo "***"
echo "***"
echo "*** Connectr first-time configuration"
echo "***"
echo "***"
echo "**************************************************"
echo "**************************************************"

echo "Connectr settings will be saved in $DST"
if [[ -z "$HOME" ]]; then
   echo ""
   echo "ERROR: \$HOME environment variable is not set!"
   echo "ERROR: Connectr cannot be configured."
   echo "ERROR: You will have to manually setup connectr.ini"
   echo ""
   exit 1
fi

echo ""
echo "To use Connectr you must configure a Spotify web application."
echo "You must have a Spotify Premium account to create and use one."
echo ""
echo "Create an application at https://developer.spotify.com/my-applications/"
echo "Add this Redirect URI: http://127.0.0.1:5432"
echo ""
echo "Fill in the values from your Spotify application page below."
while [ 1 ]; do
    echo ""
    echo -n "Client ID: "
    read APP
    echo -n "Client Secret: "
    read SECRET
    echo ""
    echo "CONFIRM:"
    echo "Using Client ID: $APP"
    echo "Using Client Secret: $SECRET"
    echo
    read -r -p "Is this correct? [Y/n] " confirm
    confirm=`echo "$confirm" | tr '[:upper:]' '[:lower:]'`
    if [[ "$confirm" =~ ^(yes|y)$ ]] || [[ -z $confirm ]]; then
        break
    fi
done
cat <<EOF > "$DST"
[connectr]
port = 5432

[application]
client_id = $APP
secret = $SECRET

[presets]
# Playlist Name = spotify:playlist:uri
EOF
echo ""
echo "Wrote: $DST"
echo "Connectr is all configured!"
echo "IMPORTANT: Make sure you added 'http://127.0.0.1:5432' as a Redirect URI for your app!"
echo ""
echo ""
