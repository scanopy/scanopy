#!/bin/bash
# First parameter must be the interface name
interface=$1
if command -v ifconfig >/dev/null 2>&1
then
        ip=$(ifconfig $interface | awk '/inet addr/ {split ($2,A,":"); print A[2]}');
elif command -v ip >/dev/null 2>&1
then
        ip=$(ip a s $interface | awk -F " " '/inet / {split($2,A,/\//); print A[1]}');
else
        echo "couldnt find ip or ifconfig"
        exit 1
fi

# move all parameters to the left, removing the first parameter aka interface name
shift
/usr/local/bin/scanopy-daemon --name="$HOSTNAME-$interface" --interfaces=$interface --bind-address=$ip $@
