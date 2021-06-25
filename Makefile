

all: mini

mini:
	@python3 setup.py build_ext --inplace	
	@rm -rf ./build ./mini.c	

.PHONY: test clean

test:
	@python3 ./tests/test_mini.py

clean:
	@rm -f *.so
