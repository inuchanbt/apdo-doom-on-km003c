SECTIONS {
 .apdo_metadata ORIGIN(FLASH) + 0x420 : { KEEP(*(.apdo_metadata)); } > FLASH
}
INSERT BEFORE .text;
SECTIONS {
 .doom_arena (NOLOAD) : ALIGN(16) { *(.doom_arena); } > SRAMH
}
INSERT BEFORE .data;
ASSERT(SIZEOF(.apdo_metadata) == 64, "Invalid metadata");
ASSERT(SIZEOF(.vector_table) <= 0x420, "Vectors overlap metadata");
ASSERT(SIZEOF(.doom_arena) == 32768, "Engine arena missing");
ASSERT(__edata == ADDR(.data) + SIZEOF(.data), "Invalid startup data boundary");
