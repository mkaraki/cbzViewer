package lepton_jpeg

// #cgo LDFLAGS: -llepton_jpeg
// #include "lepton_jpeg.h"
import "C"
import (
	"bytes"
	"fmt"
	"io"
	"unsafe"
)

func DecodeLepton(writer io.Writer, reader io.Reader) error {
	// Using io.Copy method because using reader.Read([]byte) didn't copy anything. (Return 0 bytes)
	inputData := &bytes.Buffer{}
	size, err := io.Copy(inputData, reader)
	if err != nil {
		return err
	}

	input := inputData.Bytes()

	if size == 0 {
		return fmt.Errorf("lepton input is 0 length")
	}

	prepSize := size * 3
	output := make([]byte, prepSize)

	var actualSize uint64 = 0

	resCode := C.WrapperDecompressImage(
		(*C.uchar)(unsafe.Pointer(&input[0])),
		C.ulong(uint64(size)),
		(*C.uchar)(unsafe.Pointer(&output[0])),
		C.ulong(uint64(prepSize)),
		C.int(1), // Now using 1 thread because Go uses multiple thread for http handling
		// and our program may request 2 or more (3 in dev environment) for image download.
		(*C.ulong)(unsafe.Pointer(&actualSize)),
	)

	if resCode != 0 {
		return fmt.Errorf("lepton to jpeg conversion failed with code %d", int(resCode))
	}

	_, err = writer.Write(output[:actualSize])
	if err != nil {
		return err
	}

	return nil
}
