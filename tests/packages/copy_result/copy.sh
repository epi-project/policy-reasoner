#!/bin/bash
# COPY.sh
#   by Lut99
#
# Created:
#   02 Jan 2023, 16:39:22
# Last edited:
#   02 Nov 2023, 11:27:07
# Auto updated?
#   Yes
#
# Description:
#   Defines the package code for the `copy_result` package.
# 
#   Its usage is simple. Simply provide the input in the `RESULT` environment
#   variable, and the script will copy its contents (or the file itself, if it
#   isn't a directory) to `/result`.
#
#   See `README.md` for more information.
#

# Get the input arguments by removing their quotes (since they are strings)
source=$(echo "${RESULT:q}" | jq -r '.')

# Switch on whether the input is a file or directory
if [[ -d "$source" ]]; then
    cp -rf "$source"/* "/result/"
else
    # Copy the source directly to a 'contents' file
    cp -rf "$source" "/result/contents"
fi