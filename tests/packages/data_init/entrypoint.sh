#!/bin/bash
# ENTRYPOINT.sh
#   by Lut99
#
# Created:
#   16 Jan 2023, 13:13:39
# Last edited:
#   07 Dec 2023, 11:35:46
# Auto updated?
#   Yes
#
# Description:
#   Simple script that handles parsing the input and call the correct Python
#   file.
#


# Read the command
if [[ "$#" -ne 1 ]]; then
    2>&1 echo "Usage: $0 <FUNCTION>"
    exit 1
fi
func="$1"

# Read the kind
kind=$(echo "$KIND" | python3 -c "import json, sys; print(json.load(sys.stdin))")

# Match on the function
if [[ "$func" == "zeroes" ]]; then
    # Read the number from the environment variables
    num=$NUMBER

    # Call the python script
    python3 ./zeroes.py "$num" "$kind"

else
    2>&1 echo "Unknown function '$func'"
    exit 1
fi
