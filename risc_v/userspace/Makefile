CROSS=riscv64-unknown-elf-
CXX=g++
CXXFLAGS=-Wall -O3 -static -I.
SOURCES=$(wildcard *.cpp)
OUT=$(patsubst %.cpp,%,$(SOURCES))

all: $(OUT)


%: %.cpp Makefile
	$(CROSS)$(CXX) $(CXXFLAGS) -o $@ $<


clean:
	rm -f $(OUT)
