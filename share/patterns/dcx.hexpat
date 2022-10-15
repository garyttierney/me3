struct CompressionSizes {
  char section_id[4];
  if (section_id != "DCS") {
    break;
  }

  be u32 uncompressed_size;
  be u32 compressed_size;
};

struct CompressionParameters {
  char section_id[4];
  if (section_id != "DCP") {
    break;
  }

  char algorithm[4];
  be u32 algorithm_params_len;

  if (algorithm == "DFLT" || algorithm == "KRAK") {
    u8 compression_level;
    padding[3];
  }
};

struct CompressionAssociatedData {
  char section_id[4];
  if (section_id != "DCA") {
    break;
  }

  be u32 associated_data_len;
};

struct DCX {
  char ident[4];
  if (ident != "DCX") {
    break;
  }

  be u32 version;
  CompressionSizes *sizes : be u32;
  CompressionParameters *params : be u32;
  CompressionAssociatedData *assoc_data : be u32;
  u8 *data[sizes.compressed_size] : be u32;
};

DCX dcx @0x00;
