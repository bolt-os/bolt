#!/bin/sh
set -ex
mkdir -p $PDIR

# Only make one API call a day, unless we don't have it already.
[ -d $PDIR/opensbi ] && [ "$(find $PDIR/.last-check -atime -1)" ] && exit
touch $PDIR/.last-check

# Get the latest release info
RELEASE_INFO=$PDIR/.latest-release.json
curl -Ls https://api.github.com/repos/riscv-software-src/opensbi/releases/latest >| $RELEASE_INFO

# Check if we already have a current version
TAG_NAME="$(cat $RELEASE_INFO | jq -r '.tag_name')"
[ -d $PDIR/opensbi ] && [ $(cat $PDIR/.tag-name) = $TAG_NAME ] && exit

# Download the latest release
ASSET_NAME="$(cat $RELEASE_INFO | jq -r '.assets[0].name')"
DOWNLOAD_URL="$(cat $RELEASE_INFO | jq -r '.assets[0].browser_download_url')"
! rm -rf $PDIR >/dev/null 2>&1
mkdir -p $PDIR
curl -Lo $PDIR/$ASSET_NAME $DOWNLOAD_URL
mkdir $PDIR/opensbi; tar -xf $PDIR/$ASSET_NAME -C $PDIR/opensbi || rm -rf $PDIR/opensbi

# Update .tag-name and recreate .last-check
echo "$TAG_NAME\c" >| $PDIR/.tag-name
touch $PDIR/.last-check
