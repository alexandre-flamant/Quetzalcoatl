# Quetzalcoatl

Quetzalcoatl is an SSH-based utility to convert and upload files on the Remarkable 2 tablet to accomodate the Xochitl file system.
The main goal is to bypass the use of cloud and web-usb interface which are both lacking when it comes to big files.

## Functionnality
### File type
Here are the expected supported files type:

| Type | Supported |
|:--: |:--: |
|  _pdf_ | ✅ |
| _epub_ | ❌ |
|  _cbr_ | ❌ |
|  _cbz_ | ❌ |

### Actions
Expected supported actions are
| Action | Supported |
|:--: | :--: |
| _Type conversion_ | ❌ |
| _Upload_ | ✅ |

## Compatibility
This program is developped and tested on the **3.13.1.1** version of the firmware.

## Warning and note
This software is developped as it with no warranty. I won't be liable for any dammage on your remarkable resulting from the use of it.

In case anything is broken, this software only add files in ```~/.local/share/remarkable/xochitl``` directory. SSH to the device and removing the files should fix any issues.

This is my first JS project, caution is advised on the quality of the code and the absence of bugs.
