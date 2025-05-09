MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  BOOT2                             : ORIGIN = 0x10000000, LENGTH = 0x100
  CONFIG                            : ORIGIN = ORIGIN(BOOT2) + LENGTH(BOOT2), LENGTH = 3M
  FLASH                             : ORIGIN = ORIGIN(CONFIG) + LENGTH(CONFIG), LENGTH = 100K - 0x100
  BOOTLOADER_STATE                  : ORIGIN = ORIGIN(FLASH) + LENGTH(FLASH), LENGTH = 4K
  ACTIVE                            : ORIGIN = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE), LENGTH = 6M
  DFU                               : ORIGIN = ORIGIN(ACTIVE) + LENGTH(ACTIVE), LENGTH = 6M

  RAM   : ORIGIN = 0x20000000, LENGTH = 512K
  SRAM4 : ORIGIN = 0x20080000, LENGTH = 4K
  SRAM5 : ORIGIN = 0x20081000, LENGTH = 4K
}

/*
access theese values from rust
extern "C" {
    static __config_start: u32;
    static __config_end: u32;
}
*/
__config_start = ORIGIN(CONFIG) - ORIGIN(BOOT2);
__config_end = ORIGIN(CONFIG) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_state_start = ORIGIN(BOOTLOADER_STATE) - ORIGIN(BOOT2);
__bootloader_state_end = ORIGIN(BOOTLOADER_STATE) + LENGTH(BOOTLOADER_STATE) - ORIGIN(BOOT2);

__bootloader_active_start = ORIGIN(ACTIVE) - ORIGIN(BOOT2);
__bootloader_active_end = ORIGIN(ACTIVE) + LENGTH(ACTIVE) - ORIGIN(BOOT2);

__bootloader_dfu_start = ORIGIN(DFU) - ORIGIN(BOOT2);
__bootloader_dfu_end = ORIGIN(DFU) + LENGTH(DFU) - ORIGIN(BOOT2);
