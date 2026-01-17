#!/bin/bash

# --- CONFIGURATION ---

# 1. The list of sources from your code
SOURCES=(
    "kernel"
    "mcelog"
    "edac"
    "smartd"
    "thermald"
    "rasdaemon"
    "irqbalance"
)

# 2. The list of keywords from your code
#    (Converted from Rust syntax to Bash Array)
KEYWORDS=(
    # CPU/MCE
    "mce:" "machine check" "mce_record" "CPU#" "microcode"
    # Memory
    "edac" "memory error" "dimm" "ecc" "corrected error" 
    "uncorrected error" "memory_failure" "page_fault" "out of memory"
    # Disk
    "ata" "ahci" "sata" "nvme" "scsi" "sd " "sda" "sdb" "sdc" "sdd"
    "block device" "i/o error" "medium error" "sector" "read error"
    "write error" "disk error" "smart" "bad block" "reallocated"
    # PCIe
    "pcie" "pci error" "aer" "correctable error" "fatal error"
    # Network
    "link down" "link up" "carrier" "watchdog" "tx timeout" "rx error" "tx error"
    # Thermal
    "thermal" "temperature" "overheat" "throttling" "critical temp"
    # Power
    "acpi" "power" "voltage" "battery"
    # GPU
    "gpu" "drm" "radeon" "nvidia" "nouveau" "amdgpu" "intel_"
    # General
    "hardware error" "hw error" "fault" "kernel bug" "oops" "panic"
    "rcu" "hang" "segfault"
)

# --- EXECUTION ---

echo "Starting injection of ${#KEYWORDS[@]} simulated hardware errors..."
echo "Check your app now."
echo "-----------------------------------------------------"

for KEY in "${KEYWORDS[@]}"; do
    # 1. Pick a random source from your list to test source filtering
    SOURCE=${SOURCES[$RANDOM % ${#SOURCES[@]}]}

    # 2. Construct the fake message
    #    We embed the keyword clearly in the message body.
    #    We also add "TEST_SIMULATION" so you can distinguish these from real errors later.
    MESSAGE="[Hardware Error] TEST_SIMULATION: Critical failure detected. Keyword '${KEY}' found in diagnostic data."

    # 3. Inject into Journal via logger (Standard Syslog Protocol)
    #    -t sets the 'Identifier' (Source)
    #    -p sets the priority (kern.err simulates a kernel error)
    logger -t "$SOURCE" -p kern.err "$MESSAGE"

    # 4. (Optional) Inject directly into Kernel Ring Buffer (/dev/kmsg)
    #    This is required if your app reads /dev/kmsg or `dmesg` directly.
    #    Only works if running as root.
    if [ "$EUID" -eq 0 ]; then
        # <3> is the log level for KERN_ERR
        echo "<3>${SOURCE}: ${MESSAGE}" > /dev/kmsg
    fi

    echo "Injected: [$SOURCE] ... ${KEY}"

    # 5. Sleep briefly to avoid hitting journald burst limits
    sleep 0.1
done

echo "-----------------------------------------------------"
echo "Injection complete."
