using System;
using System.Collections.Generic;
using System.Text;
using System.Threading;

namespace CSMath
{
	/// <summary>
	/// 4x4 Matrix
	/// </summary>
	/// <remarks>
	/// Matrix organization: <br/>
	/// |m00|m10|m20|m30| <br/>
	/// |m01|m11|m21|m31| <br/>
	/// |m02|m12|m22|m32| <br/>
	/// |m03|m13|m23|m33| <br/>
	/// </remarks>
	public partial struct Matrix4
	{
		/// <summary>
		/// 4-dimensional identity matrix.
		/// </summary>
		public static readonly Matrix4 Identity = new Matrix4(
			1.0, 0.0, 0.0, 0.0,
			0.0, 1.0, 0.0, 0.0,
			0.0, 0.0, 1.0, 0.0,
			0.0, 0.0, 0.0, 1.0);

		/// <summary>
		/// 4-dimensional zero matrix.
		/// </summary>
		public static readonly Matrix4 Zero = new Matrix4(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);

		#region Public Fields

		/// <summary>
		/// Value at column 0, row 0 of the matrix.
		/// </summary>
		public double M00;

		/// <summary>
		/// Value at column 0, row 1 of the matrix.
		/// </summary>
		public double M01;

		/// <summary>
		/// Value at column 0, row 2 of the matrix.
		/// </summary>
		public double M02;

		/// <summary>
		/// Value at column 0, row 3 of the matrix.
		/// </summary>
		public double M03;

		/// <summary>
		/// Value at column 1, row 0 of the matrix.
		/// </summary>
		public double M10;

		/// <summary>
		/// Value at column 1, row 1 of the matrix.
		/// </summary>
		public double M11;

		/// <summary>
		/// Value at column 1, row 2 of the matrix.
		/// </summary>
		public double M12;

		/// <summary>
		/// Value at column 1, row 3 of the matrix.
		/// </summary>
		public double M13;

		/// <summary>
		/// Value at column 2, row 0 of the matrix.
		/// </summary>
		public double M20;

		/// <summary>
		/// Value at column 2, row 1 of the matrix.
		/// </summary>
		public double M21;

		/// <summary>
		/// Value at column 2, row 2 of the matrix.
		/// </summary>
		public double M22;

		/// <summary>
		/// Value at column 2, row 3 of the matrix.
		/// </summary>
		public double M23;

		/// <summary>
		/// Value at column 3, row 0 of the matrix.
		/// </summary>
		public double M30;

		/// <summary>
		/// Value at column 3, row 1 of the matrix.
		/// </summary>
		public double M31;

		/// <summary>
		/// Value at column 3, row 2 of the matrix.
		/// </summary>
		public double M32;

		/// <summary>
		/// Value at column 3, row 3 of the matrix.
		/// </summary>
		public double M33;

		public double this[int index]
		{
			get
			{
				switch (index)
				{
					case 0:
						return this.M00;
					case 1:
						return this.M01;
					case 2:
						return this.M02;
					case 3:
						return this.M03;
					case 4:
						return this.M10;
					case 5:
						return this.M11;
					case 6:
						return this.M12;
					case 7:
						return this.M13;
					case 8:
						return this.M20;
					case 9:
						return this.M21;
					case 10:
						return this.M22;
					case 11:
						return this.M23;
					case 12:
						return this.M30;
					case 13:
						return this.M31;
					case 14:
						return this.M32;
					case 15:
						return this.M33;
					default:
						throw new IndexOutOfRangeException();
				}
			}
			set
			{
				switch (index)
				{
					case 0:
						this.M00 = value;
						break;
					case 1:
						this.M01 = value;
						break;
					case 2:
						this.M02 = value;
						break;
					case 3:
						this.M03 = value;
						break;
					case 4:
						this.M10 = value;
						break;
					case 5:
						this.M11 = value;
						break;
					case 6:
						this.M12 = value;
						break;
					case 7:
						this.M13 = value;
						break;
					case 8:
						this.M20 = value;
						break;
					case 9:
						this.M21 = value;
						break;
					case 10:
						this.M22 = value;
						break;
					case 11:
						this.M23 = value;
						break;
					case 12:
						this.M30 = value;
						break;
					case 13:
						this.M31 = value;
						break;
					case 14:
						this.M32 = value;
						break;
					case 15:
						this.M33 = value;
						break;
					default:
						throw new IndexOutOfRangeException();
				}
			}
		}

		public double this[int column, int row]
		{
			get
			{
				return this[(column * 4) + row];
			}
			set
			{
				this[(column * 4) + row] = value;
			}
		}

		#endregion Public Fields

		public Matrix4(
			double m00, double m10, double m20, double m30,
			double m01, double m11, double m21, double m31,
			double m02, double m12, double m22, double m32,
			double m03, double m13, double m23, double m33)
		{
			//Col 0
			this.M00 = m00;
			this.M01 = m01;
			this.M02 = m02;
			this.M03 = m03;

			//Col 1
			this.M10 = m10;
			this.M11 = m11;
			this.M12 = m12;
			this.M13 = m13;

			//Col 2
			this.M20 = m20;
			this.M21 = m21;
			this.M22 = m22;
			this.M23 = m23;

			//Col 3
			this.M30 = m30;
			this.M31 = m31;
			this.M32 = m32;
			this.M33 = m33;
		}

		public Matrix4(double[] elements) : this(
			elements[0], elements[1], elements[2], elements[3],
			elements[4], elements[5], elements[6], elements[7],
			elements[8], elements[9], elements[10], elements[11],
			elements[12], elements[13], elements[14], elements[15])
		{ }

		/// <summary>
		/// Creates a matrix that rotates around an arbitrary vector.
		/// </summary>
		/// <param name="axis">The axis to rotate around.</param>
		/// <param name="angle">The angle to rotate around the given axis, in radians.</param>
		/// <returns>The rotation matrix.</returns>
		public static Matrix4 CreateFromAxisAngle(XYZ axis, double angle)
		{
			// a: angle
			// x, y, z: unit vector for axis.
			//
			// Rotation matrix M can compute by using below equation.
			//
			//        T               T
			//  M = uu + (cos a)( I-uu ) + (sin a)S
			//
			// Where:
			//
			//  u = ( x, y, z )
			//
			//      [  0 -z  y ]
			//  S = [  z  0 -x ]
			//      [ -y  x  0 ]
			//
			//      [ 1 0 0 ]
			//  I = [ 0 1 0 ]
			//      [ 0 0 1 ]
			//
			//
			//     [  xx+cosa*(1-xx)   yx-cosa*yx-sina*z zx-cosa*xz+sina*y ]
			// M = [ xy-cosa*yx+sina*z    yy+cosa(1-yy)  yz-cosa*yz-sina*x ]
			//     [ zx-cosa*zx-sina*y zy-cosa*zy+sina*x   zz+cosa*(1-zz)  ]
			//
			axis = axis.Normalize();
			double x = axis.X, y = axis.Y, z = axis.Z;
			double sa = (double)Math.Sin(angle), ca = (double)Math.Cos(angle);
			double xx = x * x, yy = y * y, zz = z * z;
			double xy = x * y, xz = x * z, yz = y * z;

			Matrix4 result = new Matrix4();

			result.M00 = xx + ca * (1.0f - xx);
			result.M01 = xy - ca * xy + sa * z;
			result.M02 = xz - ca * xz - sa * y;
			result.M03 = 0.0f;
			result.M10 = xy - ca * xy - sa * z;
			result.M11 = yy + ca * (1.0f - yy);
			result.M12 = yz - ca * yz + sa * x;
			result.M13 = 0.0f;
			result.M20 = xz - ca * xz + sa * y;
			result.M21 = yz - ca * yz - sa * x;
			result.M22 = zz + ca * (1.0f - zz);
			result.M23 = 0.0f;
			result.M30 = 0.0f;
			result.M31 = 0.0f;
			result.M32 = 0.0f;
			result.M33 = 1.0f;

			return result;
		}

		/// <summary>
		/// Creates a rotation matrix from the given Quaternion rotation value.
		/// </summary>
		/// <param name="quaternion">The source Quaternion.</param>
		/// <returns>The rotation matrix.</returns>
		public static Matrix4 CreateFromQuaternion(Quaternion quaternion)
		{
			Matrix4 result;

			double xx = quaternion.X * quaternion.X;
			double yy = quaternion.Y * quaternion.Y;
			double zz = quaternion.Z * quaternion.Z;

			double xy = quaternion.X * quaternion.Y;
			double wz = quaternion.Z * quaternion.W;
			double xz = quaternion.Z * quaternion.X;
			double wy = quaternion.Y * quaternion.W;
			double yz = quaternion.Y * quaternion.Z;
			double wx = quaternion.X * quaternion.W;

			result.M00 = 1.0f - 2.0f * (yy + zz);
			result.M01 = 2.0f * (xy + wz);
			result.M02 = 2.0f * (xz - wy);
			result.M03 = 0.0f;
			result.M10 = 2.0f * (xy - wz);
			result.M11 = 1.0f - 2.0f * (zz + xx);
			result.M12 = 2.0f * (yz + wx);
			result.M13 = 0.0f;
			result.M20 = 2.0f * (xz + wy);
			result.M21 = 2.0f * (yz - wx);
			result.M22 = 1.0f - 2.0f * (yy + xx);
			result.M23 = 0.0f;
			result.M30 = 0.0f;
			result.M31 = 0.0f;
			result.M32 = 0.0f;
			result.M33 = 1.0f;

			return result;
		}

		public static Matrix4 CreateRotationMatrix(XYZ angles)
		{
			return CreateRotationMatrix(angles.X, angles.Y, angles.Z);
		}

		/// <summary>
		/// Creates a rotation matrix.
		/// </summary>
		/// <param name="x"></param>
		/// <param name="y"></param>
		/// <param name="z"></param>
		/// <returns></returns>
		public static Matrix4 CreateRotationMatrix(double x, double y, double z)
		{
			double cosx = Math.Cos(x);
			double cosy = Math.Cos(y);
			double cosz = Math.Cos(z);

			double sinx = Math.Sin(x);
			double siny = Math.Sin(y);
			double sinz = Math.Sin(z);

			//X rotation
			Matrix4 rx = new Matrix4(
				1, 0, 0, 0,
				0, cosx, sinx, 0,
				0, -sinx, cosx, 0,
				0, 0, 0, 1);

			//Y rotation
			Matrix4 ry = new Matrix4(
				cosy, 0, -siny, 0,
				0, 1, 0, 0,
				siny, 0, cosy, 0,
				0, 0, 0, 1);

			//Z rotation
			Matrix4 rz = new Matrix4(
				cosz, -sinz, 0, 0,
				sinz, cosz, 0, 0,
				0, 0, 1, 0,
				0, 0, 0, 1);

			return rx * ry * rz;
		}

		/// <summary>
		/// Creates a scaling matrix.
		/// </summary>
		/// <param name="scale">The vector containing the amount to scale by on each axis.</param>
		/// <returns>The scaling matrix.</returns>
		public static Matrix4 CreateScale(XYZ scale)
		{
			return CreateScale(scale, XYZ.Zero);
		}

		/// <summary>
		/// Creates a scaling matrix with a center point.
		/// </summary>
		/// <param name="scale">The vector containing the amount to scale by on each axis.</param>
		/// <param name="centerPoint">The center point.</param>
		/// <returns>The scaling matrix.</returns>
		public static Matrix4 CreateScale(XYZ scale, XYZ centerPoint)
		{
			Matrix4 result;

			double tx = centerPoint.X * (1 - scale.X);
			double ty = centerPoint.Y * (1 - scale.Y);
			double tz = centerPoint.Z * (1 - scale.Z);

			result.M00 = scale.X;
			result.M01 = 0.0f;
			result.M02 = 0.0f;
			result.M03 = 0.0f;
			result.M10 = 0.0f;
			result.M11 = scale.Y;
			result.M12 = 0.0f;
			result.M13 = 0.0f;
			result.M20 = 0.0f;
			result.M21 = 0.0f;
			result.M22 = scale.Z;
			result.M23 = 0.0f;
			result.M30 = tx;
			result.M31 = ty;
			result.M32 = tz;
			result.M33 = 1.0f;

			return result;
		}

		/// <summary>
		/// Creates a uniform scaling matrix that scales equally on each axis.
		/// </summary>
		/// <param name="scale">The uniform scaling factor.</param>
		/// <returns>The scaling matrix.</returns>
		public static Matrix4 CreateScale(double scale)
		{
			return CreateScale(scale, XYZ.Zero);
		}

		/// <summary>
		/// Creates a uniform scaling matrix that scales equally on each axis with a center point.
		/// </summary>
		/// <param name="scale">The uniform scaling factor.</param>
		/// <param name="centerPoint">The center point.</param>
		/// <returns>The scaling matrix.</returns>
		public static Matrix4 CreateScale(double scale, XYZ centerPoint)
		{
			return CreateScale(new XYZ(scale), centerPoint);
		}

		/// <summary>
		/// Builds a matrix that scales along the x-axis, y-axis, and z-axis.
		/// </summary>
		public static Matrix4 CreateScalingMatrix(double x, double y, double z)
		{
			return new Matrix4(
				x, 0.0, 0.0, 0.0,
				0.0, y, 0.0, 0.0,
				0.0, 0.0, z, 0.0,
				0.0, 0.0, 0.0, 1.0);
		}

		/// <summary>
		/// Creates a translation matrix.
		/// </summary>
		/// <param name="position">The amount to translate in each axis.</param>
		/// <returns>The translation matrix.</returns>
		public static Matrix4 CreateTranslation(XYZ position)
		{
			return CreateTranslation(position.X, position.Y, position.Z);
		}

		/// <summary>
		/// Creates a translation matrix.
		/// </summary>
		/// <param name="xPosition">The amount to translate on the X-axis.</param>
		/// <param name="yPosition">The amount to translate on the Y-axis.</param>
		/// <param name="zPosition">The amount to translate on the Z-axis.</param>
		/// <returns>The translation matrix.</returns>
		public static Matrix4 CreateTranslation(double xPosition, double yPosition, double zPosition)
		{
			Matrix4 result;

			result.M00 = 1.0f;
			result.M01 = 0.0f;
			result.M02 = 0.0f;
			result.M03 = 0.0f;
			result.M10 = 0.0f;
			result.M11 = 1.0f;
			result.M12 = 0.0f;
			result.M13 = 0.0f;
			result.M20 = 0.0f;
			result.M21 = 0.0f;
			result.M22 = 1.0f;
			result.M23 = 0.0f;

			result.M30 = xPosition;
			result.M31 = yPosition;
			result.M32 = zPosition;
			result.M33 = 1.0f;

			return result;
		}

		/// <summary>
		/// Gets the rotation matrix from the normal vector.
		/// </summary>
		/// <param name="zaxis">Normal vector.</param>
		/// <returns>Rotation matrix.</returns>
		public static Matrix4 GetArbitraryAxis(XYZ zaxis)
		{
			zaxis = zaxis.Normalize();

			if (zaxis.Equals(XYZ.AxisZ))
			{
				return Identity;
			}
			else if (zaxis.Equals(-XYZ.AxisZ))
			{
				return GetArbitraryAxis(-XYZ.AxisX, zaxis);
			}

			XYZ xaxis;
			if (Math.Abs(zaxis.X) < (1 / 64) && Math.Abs(zaxis.Y) < (1 / 64))
			{
				xaxis = XYZ.Cross(XYZ.AxisY, zaxis);
			}
			else
			{
				xaxis = XYZ.Cross(XYZ.AxisZ, zaxis);
			}

			xaxis = xaxis.Normalize();

			return GetArbitraryAxis(xaxis, zaxis);
		}

		/// <summary>
		/// Gets the rotation matrix from the normal vector.
		/// </summary>
		/// <param name="xaxis">X axis.</param>
		/// <param name="zaxis">Normal vector.</param>
		/// <returns>Rotation matrix.</returns>
		public static Matrix4 GetArbitraryAxis(XYZ xaxis, XYZ zaxis)
		{
			XYZ yaxis = XYZ.Cross(zaxis, xaxis);
			return new Matrix4(
				xaxis.X, yaxis.X, zaxis.X, 0.0,
				xaxis.Y, yaxis.Y, zaxis.Y, 0.0,
				xaxis.Z, yaxis.Z, zaxis.Z, 0.0,
				0.0, 0.0, 0.0, 1.0);
		}

		/// <summary>
		/// Attempts to calculate the inverse of the given matrix. If successful, result will contain the inverted matrix.
		/// </summary>
		/// <param name="matrix">The source matrix to invert.</param>
		/// <param name="result">If successful, contains the inverted matrix.</param>
		/// <returns>True if the source matrix could be inverted; False otherwise.</returns>
		public static bool Inverse(Matrix4 matrix, out Matrix4 result)
		{
			//                                       -1
			// If you have matrix M, inverse Matrix M   can compute
			//
			//     -1       1
			//    M   = --------- A
			//            det(M)
			//
			// A is adjugate (adjoint) of M, where,
			//
			//      T
			// A = C
			//
			// C is Cofactor matrix of M, where,
			//           i + j
			// C   = (-1)      * det(M  )
			//  ij                    ij
			//
			//     [ a b c d ]
			// M = [ e f g h ]
			//     [ i j k l ]
			//     [ m n o p ]
			//
			// First Row
			//           2 | f g h |
			// C   = (-1)  | j k l | = + ( f ( kp - lo ) - g ( jp - ln ) + h ( jo - kn ) )
			//  11         | n o p |
			//
			//           3 | e g h |
			// C   = (-1)  | i k l | = - ( e ( kp - lo ) - g ( ip - lm ) + h ( io - km ) )
			//  12         | m o p |
			//
			//           4 | e f h |
			// C   = (-1)  | i j l | = + ( e ( jp - ln ) - f ( ip - lm ) + h ( in - jm ) )
			//  13         | m n p |
			//
			//           5 | e f g |
			// C   = (-1)  | i j k | = - ( e ( jo - kn ) - f ( io - km ) + g ( in - jm ) )
			//  14         | m n o |
			//
			// Second Row
			//           3 | b c d |
			// C   = (-1)  | j k l | = - ( b ( kp - lo ) - c ( jp - ln ) + d ( jo - kn ) )
			//  21         | n o p |
			//
			//           4 | a c d |
			// C   = (-1)  | i k l | = + ( a ( kp - lo ) - c ( ip - lm ) + d ( io - km ) )
			//  22         | m o p |
			//
			//           5 | a b d |
			// C   = (-1)  | i j l | = - ( a ( jp - ln ) - b ( ip - lm ) + d ( in - jm ) )
			//  23         | m n p |
			//
			//           6 | a b c |
			// C   = (-1)  | i j k | = + ( a ( jo - kn ) - b ( io - km ) + c ( in - jm ) )
			//  24         | m n o |
			//
			// Third Row
			//           4 | b c d |
			// C   = (-1)  | f g h | = + ( b ( gp - ho ) - c ( fp - hn ) + d ( fo - gn ) )
			//  31         | n o p |
			//
			//           5 | a c d |
			// C   = (-1)  | e g h | = - ( a ( gp - ho ) - c ( ep - hm ) + d ( eo - gm ) )
			//  32         | m o p |
			//
			//           6 | a b d |
			// C   = (-1)  | e f h | = + ( a ( fp - hn ) - b ( ep - hm ) + d ( en - fm ) )
			//  33         | m n p |
			//
			//           7 | a b c |
			// C   = (-1)  | e f g | = - ( a ( fo - gn ) - b ( eo - gm ) + c ( en - fm ) )
			//  34         | m n o |
			//
			// Fourth Row
			//           5 | b c d |
			// C   = (-1)  | f g h | = - ( b ( gl - hk ) - c ( fl - hj ) + d ( fk - gj ) )
			//  41         | j k l |
			//
			//           6 | a c d |
			// C   = (-1)  | e g h | = + ( a ( gl - hk ) - c ( el - hi ) + d ( ek - gi ) )
			//  42         | i k l |
			//
			//           7 | a b d |
			// C   = (-1)  | e f h | = - ( a ( fl - hj ) - b ( el - hi ) + d ( ej - fi ) )
			//  43         | i j l |
			//
			//           8 | a b c |
			// C   = (-1)  | e f g | = + ( a ( fk - gj ) - b ( ek - gi ) + c ( ej - fi ) )
			//  44         | i j k |
			//
			// Cost of operation
			// 53 adds, 104 muls, and 1 div.
			double a = matrix.M00, b = matrix.M01, c = matrix.M02, d = matrix.M03;
			double e = matrix.M10, f = matrix.M11, g = matrix.M12, h = matrix.M13;
			double i = matrix.M20, j = matrix.M21, k = matrix.M22, l = matrix.M23;
			double m = matrix.M30, n = matrix.M31, o = matrix.M32, p = matrix.M33;

			double kp_lo = k * p - l * o;
			double jp_ln = j * p - l * n;
			double jo_kn = j * o - k * n;
			double ip_lm = i * p - l * m;
			double io_km = i * o - k * m;
			double in_jm = i * n - j * m;

			double a11 = +(f * kp_lo - g * jp_ln + h * jo_kn);
			double a12 = -(e * kp_lo - g * ip_lm + h * io_km);
			double a13 = +(e * jp_ln - f * ip_lm + h * in_jm);
			double a14 = -(e * jo_kn - f * io_km + g * in_jm);

			double det = a * a11 + b * a12 + c * a13 + d * a14;

			if (Math.Abs(det) < double.Epsilon)
			{
				result = new Matrix4(double.NaN, double.NaN, double.NaN, double.NaN,
									   double.NaN, double.NaN, double.NaN, double.NaN,
									   double.NaN, double.NaN, double.NaN, double.NaN,
									   double.NaN, double.NaN, double.NaN, double.NaN);
				return false;
			}

			double invDet = 1.0f / det;

			result.M00 = a11 * invDet;
			result.M10 = a12 * invDet;
			result.M20 = a13 * invDet;
			result.M30 = a14 * invDet;

			result.M01 = -(b * kp_lo - c * jp_ln + d * jo_kn) * invDet;
			result.M11 = +(a * kp_lo - c * ip_lm + d * io_km) * invDet;
			result.M21 = -(a * jp_ln - b * ip_lm + d * in_jm) * invDet;
			result.M31 = +(a * jo_kn - b * io_km + c * in_jm) * invDet;

			double gp_ho = g * p - h * o;
			double fp_hn = f * p - h * n;
			double fo_gn = f * o - g * n;
			double ep_hm = e * p - h * m;
			double eo_gm = e * o - g * m;
			double en_fm = e * n - f * m;

			result.M02 = +(b * gp_ho - c * fp_hn + d * fo_gn) * invDet;
			result.M12 = -(a * gp_ho - c * ep_hm + d * eo_gm) * invDet;
			result.M22 = +(a * fp_hn - b * ep_hm + d * en_fm) * invDet;
			result.M32 = -(a * fo_gn - b * eo_gm + c * en_fm) * invDet;

			double gl_hk = g * l - h * k;
			double fl_hj = f * l - h * j;
			double fk_gj = f * k - g * j;
			double el_hi = e * l - h * i;
			double ek_gi = e * k - g * i;
			double ej_fi = e * j - f * i;

			result.M03 = -(b * gl_hk - c * fl_hj + d * fk_gj) * invDet;
			result.M13 = +(a * gl_hk - c * el_hi + d * ek_gi) * invDet;
			result.M23 = -(a * fl_hj - b * el_hi + d * ej_fi) * invDet;
			result.M33 = +(a * fk_gj - b * ek_gi + c * ej_fi) * invDet;

			return true;
		}

		/// <inheritdoc/>
		public override bool Equals(object obj)
		{
			if (!(obj is Matrix4 other))
				return false;

			return this.M00 == other.M00 && this.M11 == other.M11 && this.M22 == other.M22 && this.M33 == other.M33 &&
				this.M01 == other.M01 && this.M02 == other.M02 && this.M03 == other.M03 &&
				this.M10 == other.M10 && this.M12 == other.M12 && this.M13 == other.M13 &&
				this.M20 == other.M20 && this.M21 == other.M21 && this.M23 == other.M23 &&
				this.M30 == other.M30 && this.M31 == other.M31 && this.M32 == other.M32;
		}

		/// <summary>
		/// Gets the matrix columns.
		/// </summary>
		/// <returns></returns>
		public List<XYZM> GetCols()
		{
			return new List<XYZM>
			{
				new XYZM(this.M00, this.M01, this.M02, this.M03),
				new XYZM(this.M10, this.M11, this.M12, this.M13),
				new XYZM(this.M20, this.M21, this.M22, this.M23),
				new XYZM(this.M30, this.M31, this.M32, this.M33)
			};
		}

		/// <summary>
		/// Calculates the determinant of the matrix.
		/// </summary>
		/// <returns>The determinant of the matrix.</returns>
		public double GetDeterminant()
		{
			// | a b c d |     | f g h |     | e g h |     | e f h |     | e f g |
			// | e f g h | = a | j k l | - b | i k l | + c | i j l | - d | i j k |
			// | i j k l |     | n o p |     | m o p |     | m n p |     | m n o |
			// | m n o p |
			//
			//   | f g h |
			// a | j k l | = a ( f ( kp - lo ) - g ( jp - ln ) + h ( jo - kn ) )
			//   | n o p |
			//
			//   | e g h |
			// b | i k l | = b ( e ( kp - lo ) - g ( ip - lm ) + h ( io - km ) )
			//   | m o p |
			//
			//   | e f h |
			// c | i j l | = c ( e ( jp - ln ) - f ( ip - lm ) + h ( in - jm ) )
			//   | m n p |
			//
			//   | e f g |
			// d | i j k | = d ( e ( jo - kn ) - f ( io - km ) + g ( in - jm ) )
			//   | m n o |

			double a = this.M00, b = this.M10, c = this.M20, d = this.M30;
			double e = this.M01, f = this.M11, g = this.M21, h = this.M31;
			double i = this.M02, j = this.M12, k = this.M22, l = this.M32;
			double m = this.M03, n = this.M13, o = this.M23, p = this.M33;

			double kp_lo = k * p - l * o;
			double jp_ln = j * p - l * n;
			double jo_kn = j * o - k * n;
			double ip_lm = i * p - l * m;
			double io_km = i * o - k * m;
			double in_jm = i * n - j * m;

			return a * (f * kp_lo - g * jp_ln + h * jo_kn) -
				   b * (e * kp_lo - g * ip_lm + h * io_km) +
				   c * (e * jp_ln - f * ip_lm + h * in_jm) -
				   d * (e * jo_kn - f * io_km + g * in_jm);
		}

		/// <inheritdoc/>
		public override int GetHashCode()
		{
			return this.M00.GetHashCode() + this.M01.GetHashCode() + this.M02.GetHashCode() + this.M03.GetHashCode() +
				   this.M10.GetHashCode() + this.M11.GetHashCode() + this.M12.GetHashCode() + this.M13.GetHashCode() +
				   this.M20.GetHashCode() + this.M21.GetHashCode() + this.M22.GetHashCode() + this.M23.GetHashCode() +
				   this.M30.GetHashCode() + this.M31.GetHashCode() + this.M32.GetHashCode() + this.M33.GetHashCode();
		}

		/// <summary>
		/// Gets the matrix rows.
		/// </summary>
		/// <returns></returns>
		public List<XYZM> GetRows()
		{
			return new List<XYZM>
			{
				new XYZM(this.M00, this.M10, this.M20, this.M30),
				new XYZM(this.M01, this.M11, this.M21, this.M31),
				new XYZM(this.M02, this.M12, this.M22, this.M32),
				new XYZM(this.M03, this.M13, this.M23, this.M33)
			};
		}

		/// <inheritdoc/>
		public override string ToString()
		{
			string separator = Thread.CurrentThread.CurrentCulture.TextInfo.ListSeparator;
			StringBuilder s = new StringBuilder();
			s.Append(string.Format("|{0}{4} {1}{4} {2}{4} {3}|" + Environment.NewLine, this.M00, this.M01, this.M02, this.M03, separator));
			s.Append(string.Format("|{0}{4} {1}{4} {2}{4} {3}|" + Environment.NewLine, this.M10, this.M11, this.M12, this.M13, separator));
			s.Append(string.Format("|{0}{4} {1}{4} {2}{4} {3}|" + Environment.NewLine, this.M20, this.M21, this.M22, this.M23, separator));
			s.Append(string.Format("|{0}{4} {1}{4} {2}{4} {3}|", this.M30, this.M31, this.M32, this.M33, separator));
			return s.ToString();
		}

		/// <summary>
		/// Transposes the rows and columns of this matrix.
		/// </summary>
		/// <returns>The transposed matrix.</returns>
		public Matrix4 Transpose()
		{
			Matrix4 result;

			result.M00 = this.M00;
			result.M01 = this.M10;
			result.M02 = this.M20;
			result.M03 = this.M30;
			result.M10 = this.M01;
			result.M11 = this.M11;
			result.M12 = this.M21;
			result.M13 = this.M31;
			result.M20 = this.M02;
			result.M21 = this.M12;
			result.M22 = this.M22;
			result.M23 = this.M32;
			result.M30 = this.M03;
			result.M31 = this.M13;
			result.M32 = this.M23;
			result.M33 = this.M33;

			return result;
		}
	}
}