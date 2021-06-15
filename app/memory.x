/* Linker script for the nRF52 - WITHOUT SOFT DEVICE */
MEMORY
{
  /* NOTE K = KiBi = 1024 bytes */
  FLASH : ORIGIN = 0x20020000, LENGTH = 64K
  RAM :   ORIGIN = 0x20030000, LENGTH = 64K
}
