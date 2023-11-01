#!/bin/bash

# Implements the simplest script ever with two functions; it simply checks if the dataset exists and then outputs '0'.

# Read the function
if [[ "$#" -ne 1 ]]; then
    echo "Usage: $0 <function>"
    echo ""
    echo "Possible functions are:"
    echo " - local_compute"
    echo " - aggregate"
    exit 1
else
    func="$1"
fi



# Match the function
if [[ "$func" == "local_compute" ]]; then
    # Read the input dataset from the env
    input=$(echo "$INPUT" | python3 -c "import json, sys; print(json.load(sys.stdin))")
    if [[ ! -f "$input" ]]; then
        echo "Input dataset '$input' does not exist (or is not a file)"
        exit 1
    fi

    # Write '0' to the output thing
    echo "0" >> "/result/output_local.txt"

    # No need to return; Brane will know what's up

elif [[ "$func" == "aggregate" ]]; then
    # Read the input datasets from the env
    lhs=$(echo "$LHS" | python3 -c "import json, sys; print(json.load(sys.stdin))")/output_local.txt
    rhs=$(echo "$RHS" | python3 -c "import json, sys; print(json.load(sys.stdin))")/output_local.txt
    if [[ ! -f "$lhs" ]]; then
        echo "Input dataset '$lhs' does not exist (or is not a file)"
        exit 1
    fi
    if [[ ! -f "$rhs" ]]; then
        echo "Input dataset '$rhs' does not exist (or is not a file)"
        exit 1
    fi

    # Aggregate all input things
    echo "$(cat "$lhs")$(cat "$rhs")" >> "/result/output.txt"

    # No need to return; Brane will know what's up

else
    echo "Unknown function '$func'"
    echo ""
    echo "Possible functions are:"
    echo " - local_compute"
    echo " - aggregate"
    exit 1
fi
