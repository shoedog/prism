#!/bin/sh
# Test fixture: firmware update script with taint and absence patterns

FW_IMAGE=$1
PARTITION=${2:-firmware}

# Taint: $FW_IMAGE from user input flows to mtd write
# Absence: hash check should precede mtd write

validate_image() {
    local image=$1
    if [ ! -f "$image" ]; then
        echo "Error: firmware image not found"
        return 1
    fi
    FW_HASH=$(sha256sum "$image" | cut -d' ' -f1)
    EXPECTED=$(cat /etc/firmware.sha256)
    if [ "$FW_HASH" != "$EXPECTED" ]; then
        echo "Hash mismatch"
        return 1
    fi
}

apply_config() {
    local vlan_id=$1
    local iface=$2
    # BUG: unquoted variables — command injection
    vconfig add $iface $vlan_id
    ifconfig $iface.$vlan_id up
}

do_upgrade() {
    validate_image "$FW_IMAGE" || exit 1

    echo "Writing firmware to $PARTITION..."
    mtd write "$FW_IMAGE" "$PARTITION"

    if [ $? -ne 0 ]; then
        echo "Flash write failed!"
        exit 1
    fi

    echo "Rebooting..."
    reboot
}

do_upgrade
