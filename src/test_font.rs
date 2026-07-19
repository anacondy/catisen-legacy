fn main() { for b in font8x8::BASIC_FONTS.get('C').unwrap() { println!("{:08b}", b); } }
