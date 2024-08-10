// This is header file to use lepton_jpeg_rust
// This isn't official header file and only implement functions that we need.

int WrapperDecompressImage(
    unsigned char *input_buffer,
    unsigned long input_buffer_size,
    unsigned char *output_buffer,
    unsigned long output_buffer_size,
    int number_of_threads,
    unsigned long *result_size
);