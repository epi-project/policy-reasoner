#!/usr/bin/env python3
# ZEROES.py
#   by Lut99
#
# Created:
#   16 Jan 2023, 13:16:26
# Last edited:
#   07 Dec 2023, 11:36:03
# Auto updated?
#   Yes
#
# Description:
#   Implements a generator for an intermediate result of only zeroes (ASCII
#   `0`).
#

import argparse
import sys


##### GENERATION FUNCTIONS #####
def generate_vector(n: int) -> int:
    """
        Generates a zeroes file with the vector layout.

        Specifically, generates a file /result/data with `n` zeroes, delimited
        by spaces. Every zero is simply the ASCII character for `0`.

        # Arguments
        - `n`: The number of ASCII `0` to generate.

        # Returns
        The return code of the operation. `0` means success.
    """

    # Attempt to open the output file
    try:
        with open("/result/data", "w") as h:
            # Write the number of zeroes
            for i in range(n):
                if i > 0: h.write(" ")
                h.write("0")

    except IOError as e:
        print(f"Failed to write to output file '/result/data': {e}", file=sys.stderr)
        return e.errno

    # Done
    return 0





##### ENTRYPOINT #####
def main(n: int, kind: str) -> int:
    """
        Entrypoint to the script.

        # Arguments
        - `n`: The number of zeroes to generate.
        - `kind`: The kind of dataset to generate.

        # Returns
        The exit code for the script. `0` means success.
    """

    # Match on the kind
    if kind == "vector":
        return generate_vector(n)

    # Should never happen
    raise RuntimeError(f"main() saw a non-allowed kind '{kind}' (should have been taken care of by the argument parser)")



# The actual entrypoint
if __name__ == "__main__":
    # Parse the arguments
    parser = argparse.ArgumentParser()
    parser.add_argument("NUMBER", type=int, help="The number of zeroes to generate in the file.")
    parser.add_argument("KIND", choices=["vector"], help="The kind of dataset to generate.")
    args = parser.parse_args()

    # Verify numbers is in-of-bounds
    if args.NUMBER < 0:
        print(f"NUMBER has to be a non-negative integer, not {args.NUMBER}", file=sys.stderr)
        exit(1)

    # Call main
    exit(main(args.NUMBER, args.KIND))
