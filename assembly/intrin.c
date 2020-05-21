#include <stdio.h>
#include <pmmintrin.h>

void calc_intrin(float result[], float matrix[], float vector[]);
void calc_asm(float result[], float matrix[], float vector[]);

int main() {
	int row, col;
	float vec[] = {1.0, 10.0, 100.0, 1000.0};
	float mat[] = {2.0, 0.0, 0.0, 0.0,
		       0.0, 2.2, 0.0, 0.0,
		       0.0, 0.0, 22.2, 0.0,
		       0.0, 0.0, 0.0, 22.22};

	float result[4];

	calc_intrin(result, mat, vec);

	printf("%5.3f %5.3f %5.3f %5.3f\n", result[0], result[1], result[2], result[3]);

	calc_asm(result, mat, vec);

	printf("%5.3f %5.3f %5.3f %5.3f\n", result[0], result[1], result[2], result[3]);
	
	
	return 0;
}

void calc_intrin(float result[], float matrix[], float vector[])
{
	int row;
	__m128 vec = _mm_loadu_ps(vector);
	for (row = 0;row < 4;row++) {
		__m128 rowvec = _mm_loadu_ps(&matrix[row * 4]);
		__m128 rowvec2 = _mm_mul_ps(vec, rowvec);
		__m128 rowvec3 = _mm_hadd_ps(rowvec2, rowvec2);
		__m128 rowvec4 = _mm_hadd_ps(rowvec3, rowvec3);

		_mm_store_ss(&result[row], rowvec4);
	}
}

