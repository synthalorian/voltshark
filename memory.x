MEMORY
{
  /* RP2040 — 2MB external QSPI flash, 264KB SRAM */
  BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
  FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
  RAM   : ORIGIN = 0x20000000, LENGTH = 256K
  SCRATCH_X : ORIGIN = 0x20040000, LENGTH = 4K
  SCRATCH_Y : ORIGIN = 0x20041000, LENGTH = 4K
}

/* Stack top — end of RAM */
__stack_top = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS {
    /* Second-stage bootloader is prepended to the image.
       rp2040-boot2 handles the 256-byte checksummed block. */
} INSERT BEFORE .text;
