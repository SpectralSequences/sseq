First byte is the header. The first (least significant) bit is whether Sseq is in the recipient, second is whether Resolver is in the recipient. Third bit indicates the target sseq (0 if Main, 1 if Unit). The next four bits (a single hexadecimal) give the type of the action. Currently there are 8 possible actions. F is reserved to indicate that the next byte should be used for this purpose if we have more than 16.

string is encoded as follows --- first a u8 that indicates the length of the string, then the string encoded in utf8. string16 is the same but the length is a u16

enum Algebra {
    Adem = 0,
    Milnor = 1,
}

0. Construct
?   string   module_name
1   Algebra  algebra_name

1. ConstructJson
?   string16 data
1   Algebra  algebra_name

2. Resolve
2   i8      max_degree

3. AddProductDifferential
4. AddProductType
2   i8      x
2   i8      y
1   bool    permanent
??? class
?   string  name

5. AddPermanentClass
2   i8      x
2   i8      y
??? class

First there are two numbers, (x, y), 
6. AddDifferential
2   i8      x
2   i8      y
1   u8      r
??? source
??? target

7. SetClassName
2   i8      x
2   i8      y
1   u8      idx
?   string  name
