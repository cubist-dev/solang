
        contract test {
            using ints for uint32;
            function foo(uint32 x) public pure returns (uint64) {
                // x is 32 bit but the max function takes 64 bit uint
                return x.max(65536, 2);
            }
        }

        library ints {
            uint64 constant nada = 0;

            function max(uint64 a, uint64 b) internal pure returns (uint64) {
                return a > b ? a : b;
            }
            function max(uint64 a) internal pure returns (uint64) {
                return a;
            }
        }