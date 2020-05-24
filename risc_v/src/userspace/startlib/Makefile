CROSS=riscv64-unknown-linux-gnu-
CXX=g++
OBJCOPY=objcopy
AR=ar
CXXFLAGS=-Wall -O0 -ffreestanding -nostartfiles -nostdlib -I. -march=rv64g -mabi=lp64d
OUT=libstart.a
SOURCES_S=$(wildcard *.S)
SOURCES_CPP=$(wildcard *.cpp)
OBJS=$(patsubst %.S,%.o,$(SOURCES_S)) $(patsubst %.cpp,%.o,$(SOURCES_CPP))
all: $(OUT)

$(OUT): $(OBJS) Makefile
	rm -f $(OUT)
	$(AR) rcv $(OUT) $(OBJS)

%.o: %.S
	$(CROSS)$(CXX) $(CXXFLAGS) -c $< -o $@

%.o: %.cpp
	$(CROSS)$(CXX) $(CXXFLAGS) -c $< -o $@

.PHONY: clean

clean:
	rm -f $(OUT) $(OBJS)
