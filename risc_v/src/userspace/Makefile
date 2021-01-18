CROSS=/opt/riscv/bin/riscv64-unknown-linux-gnu-
CXX=g++
OBJCOPY=objcopy
CXXFLAGS=-Wall -O3 -ffreestanding -nostartfiles -nostdlib -static -march=rv64g -mabi=lp64d
LINKER_SCRIPT=-T./startlib/linker.lds
INCLUDES=-I./startlib
LIBS=-L./startlib
LIB=-lstart
SOURCES=$(wildcard *.cpp)
OUT=$(patsubst %.cpp,%.elf,$(SOURCES))
PROGS=$(patsubst %.cpp,%,$(SOURCES))

all: $(OUT)

%.elf:%.cpp Makefile startlib/linker.lds startlib/*.h
	$(CROSS)$(CXX) $(CXXFLAGS) $(INCLUDES) $(LIBS) -o $@ $< $(LIB)
	$(CROSS)$(OBJCOPY) -O binary $@ $@.bin
	mv $@.bin $(basename $@)

clean:
	rm -f $(OUT)
	rm -f *.bin *.elf
	rm -f $(PROGS)
