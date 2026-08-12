import struct
import os

def create_valid_ico(width=32, height=32):
    # BITMAPINFOHEADER is 40 bytes
    # XOR image size = 32 * 32 * 4 = 4096 bytes
    # AND mask size = (32 / 8) * 32 = 128 bytes
    bmi_header_size = 40
    xor_size = width * height * 4
    and_size = (width // 8) * height
    image_size = bmi_header_size + xor_size + and_size

    # ICO Header (6 bytes)
    ico_header = struct.pack('<HHH', 0, 1, 1)

    # Directory Entry (16 bytes)
    # width, height, colorCount, reserved, planes, bpp, bytesInRes, imageOffset
    dir_entry = struct.pack('<BBBBHHII', width, height, 0, 0, 1, 32, image_size, 22)

    # BITMAPINFOHEADER (40 bytes) - height is double in ICO format (XOR + AND mask)
    bmi = struct.pack('<IiiHHIIiiII', 
        bmi_header_size, 
        width, 
        height * 2, 
        1, 
        32, 
        0, 
        xor_size, 
        0, 
        0, 
        0, 
        0
    )

    # XOR mask: BGRA order, bottom-up
    # Color #3b82f6 -> B=246(0xf6), G=130(0x82), R=59(0x3b), A=255(0xff)
    pixel = bytes([0xf6, 0x82, 0x3b, 0xff])
    xor_data = pixel * (width * height)

    # AND mask: 1 bit per pixel, 0 = opaque
    and_mask = b'\x00' * and_size

    return ico_header + dir_entry + bmi + xor_data + and_mask

def create_valid_png(width=32, height=32):
    import zlib
    raw_data = b''.join(b'\x00' + b'\x3b\x82\xf6\xff' * width for _ in range(height))
    compressed = zlib.compress(raw_data)
    
    def chunk(tag, data):
        return struct.pack('>I', len(data)) + tag + data + struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff)

    return (
        b'\x89PNG\r\n\x1a\n'
        + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
        + chunk(b'IDAT', compressed)
        + chunk(b'IEND', b'')
    )

os.makedirs('src-tauri/icons', exist_ok=True)

ico_data = create_valid_ico(32, 32)
png_data = create_valid_png(32, 32)

with open('src-tauri/icons/icon.ico', 'wb') as f:
    f.write(ico_data)

with open('src-tauri/icons/icon.png', 'wb') as f:
    f.write(png_data)

with open('src-tauri/icons/32x32.png', 'wb') as f:
    f.write(png_data)

print("Valid ICO and PNG icons created successfully!")
