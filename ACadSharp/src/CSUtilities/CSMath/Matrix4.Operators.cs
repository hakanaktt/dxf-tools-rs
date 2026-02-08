using System;
using System.Collections.Generic;

namespace CSMath
{
	public partial struct Matrix4
	{
		/// <summary>
		/// Multiplies two matrices.
		/// </summary>
		/// <returns>A new instance containing the result.</returns>
		public static Matrix4 Multiply(Matrix4 a, Matrix4 b)
		{
			Matrix4 result = new Matrix4();

			var rows = a.GetRows();
			var cols = b.GetCols();

			result.M00 = rows[0].Dot(cols[0]);
			result.M10 = rows[0].Dot(cols[1]);
			result.M20 = rows[0].Dot(cols[2]);
			result.M30 = rows[0].Dot(cols[3]);
			result.M01 = rows[1].Dot(cols[0]);
			result.M11 = rows[1].Dot(cols[1]);
			result.M21 = rows[1].Dot(cols[2]);
			result.M31 = rows[1].Dot(cols[3]);
			result.M02 = rows[2].Dot(cols[0]);
			result.M12 = rows[2].Dot(cols[1]);
			result.M22 = rows[2].Dot(cols[2]);
			result.M32 = rows[2].Dot(cols[3]);
			result.M03 = rows[3].Dot(cols[0]);
			result.M13 = rows[3].Dot(cols[1]);
			result.M23 = rows[3].Dot(cols[2]);
			result.M33 = rows[3].Dot(cols[3]);

			return result;
		}

		/// <summary>
		/// Multiplies two matrices.
		/// </summary>
		/// <returns>A new instance containing the result.</returns>
		public static Matrix4 operator *(Matrix4 a, Matrix4 b)
		{
			return Matrix4.Multiply(a, b);
		}

		/// <summary>Multiply the matrix and a coordinate</summary>
		/// <param name="matrix"></param>
		/// <param name="value"></param>
		/// <returns>Result matrix</returns>
		public static XYZ operator *(Matrix4 matrix, XYZ value)
		{
			XYZM xyzm = new XYZM(value.X, value.Y, value.Z, 1);
			List<XYZM> rows = matrix.GetRows();

			XYZ result = new XYZ
			{
				X = rows[0].Dot(xyzm),
				Y = rows[1].Dot(xyzm),
				Z = rows[2].Dot(xyzm)
			};

			return result;
		}

		/// <summary>Multiply the matrix and XYZM</summary>
		/// <param name="matrix"></param>
		/// <param name="v"></param>
		/// <returns>Result matrix</returns>
		public static XYZM operator *(Matrix4 matrix, XYZM v)
		{
			return new XYZM(
				matrix.M00 * v.X + matrix.M10 * v.Y + matrix.M20 * v.Z + matrix.M30 * v.M,
				matrix.M01 * v.X + matrix.M11 * v.Y + matrix.M21 * v.Z + matrix.M31 * v.M,
				matrix.M02 * v.X + matrix.M12 * v.Y + matrix.M22 * v.Z + matrix.M32 * v.M,
				matrix.M03 * v.X + matrix.M13 * v.Y + matrix.M23 * v.Z + matrix.M33 * v.M);
		}
	}
}
