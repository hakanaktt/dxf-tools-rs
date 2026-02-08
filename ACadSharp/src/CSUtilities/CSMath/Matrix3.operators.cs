namespace CSMath
{
	public partial struct Matrix3
	{
		public static XYZ operator *(Matrix3 matrix, XYZ value)
		{
			return new XYZ(matrix.M00 * value.X + matrix.M01 * value.Y + matrix.M02 * value.Z,
							matrix.M10 * value.X + matrix.M11 * value.Y + matrix.M12 * value.Z,
							matrix.M20 * value.X + matrix.M21 * value.Y + matrix.M22 * value.Z);
		}

		public static Matrix3 operator *(Matrix3 a, Matrix3 b)
		{
			return new Matrix3(a.M00 * b.M00 + a.M01 * b.M10 + a.M02 * b.M20, a.M00 * b.M01 + a.M01 * b.M11 + a.M02 * b.M21, a.M00 * b.M02 + a.M01 * b.M12 + a.M02 * b.M22,
							   a.M10 * b.M00 + a.M11 * b.M10 + a.M12 * b.M20, a.M10 * b.M01 + a.M11 * b.M11 + a.M12 * b.M21, a.M10 * b.M02 + a.M11 * b.M12 + a.M12 * b.M22,
							   a.M20 * b.M00 + a.M21 * b.M10 + a.M22 * b.M20, a.M20 * b.M01 + a.M21 * b.M11 + a.M22 * b.M21, a.M20 * b.M02 + a.M21 * b.M12 + a.M22 * b.M22);
		}
	}
}
