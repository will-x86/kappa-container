#!/bin/sh

set -e  

if ! command -v cgdelete >/dev/null 2>&1; then
    echo "Error: cgdelete not found. Please install cgroup-tools (or libcgroup-tools)"
    exit 1
fi

for cgroup in /sys/fs/cgroup/kappa-*; do
    if [ -d "$cgroup" ]; then
        cgroup_name=$(basename "$cgroup")
        echo "Processing cgroup: $cgroup_name"
        
        if [ -f "$cgroup/cgroup.procs" ]; then
            echo "Killing processes in $cgroup_name..."
            while read -r pid; do
                if [ -n "$pid" ]; then
                    echo "Killing PID: $pid"
                    kill -9 "$pid" 2>/dev/null || true
                fi
            done < "$cgroup/cgroup.procs"
        fi

        echo "" > "$cgroup/cgroup.subtree_control" 2>/dev/null || true
        
        echo "Attempting cgdelete for $cgroup_name..."
        cgdelete -g all:"$cgroup_name" 2>/dev/null || true
        
        if [ -d "$cgroup" ]; then
            echo "cgdelete failed, trying manual removal..."
            if [ -f "$cgroup/cgroup.procs" ]; then
                while read -r pid; do
                    if [ -n "$pid" ]; then
                        echo "$pid" > /sys/fs/cgroup/cgroup.procs 2>/dev/null || true
                    fi
                done < "$cgroup/cgroup.procs"
            fi
            
            rmdir "$cgroup" 2>/dev/null || true
        fi
        
        if [ -d "$cgroup" ]; then
            echo "WARNING: Failed to remove $cgroup_name"
        else
            echo "Successfully removed $cgroup_name"
        fi
    fi
done

remaining=$(find /sys/fs/cgroup -name "kappa-*" -type d 2>/dev/null | wc -l)
if [ "$remaining" -gt 0 ]; then
    echo "WARNING: $remaining kappa cgroups still remain!"
    exit 1
else
    echo "Successfully cleaned up all kappa cgroups"
fi
