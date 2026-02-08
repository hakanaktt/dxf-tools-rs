using System;
using System.Text;
using System.Threading;

namespace CSMath
{
	public partial struct Matrix3
	{
		/// <summary>
		/// 4-dimensional zero matrix.
		/// </summary>
		public static readonly Matrix3 Zero = new Matrix3(
			0.0, 0.0, 0.0,
			0.0, 0.0, 0.0,
			0.0, 0.0, 0.0);

		/// <summary>
		/// 4-dimensional identity matrix.
		/// </summary>
		public static readonly Matrix3 Identity = new Matrix3(
			1.0, 0.0, 0.0,
			0.0, 1.0, 0.0,
			0.0, 0.0, 1.0);

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

		#endregion Public Fields

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
						return this.M10;
					case 4:
						return this.M11;
					case 5:
						return this.M12;
					case 6:
						return this.M20;
					case 7:
						return this.M21;
					case 8:
						return this.M22;
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
						this.M10 = value;
						break;
					case 4:
						this.M11 = value;
						break;
					case 5:
						this.M12 = value;
						break;
					case 6:
						this.M20 = value;
						break;
					case 7:
						this.M21 = value;
						break;
					case 8:
						this.M22 = value;
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
				return this[(column * 3) + row];
			}
			set
			{
				this[(column * 3) + row] = value;
			}
		}

		public Matrix3(
			double m00, double m10, double m20,
			double m01, double m11, double m21,
			double m02, double m12, double m22)
		{
			//Col 0
			this.M00 = m00;
			this.M01 = m01;
			this.M02 = m02;

			//Col 1
			this.M10 = m10;
			this.M11 = m11;
			this.M12 = m12;

			//Col 2
			this.M20 = m20;
			this.M21 = m21;
			this.M22 = m22;
		}

		public Matrix3(Matrix4 matrix)
		{
			this.M00 = matrix.M00;
			this.M01 = matrix.M01;
			this.M02 = matrix.M02;
			this.M10 = matrix.M10;
			this.M11 = matrix.M11;
			this.M12 = matrix.M12;
			this.M20 = matrix.M20;
			this.M21 = matrix.M21;
			this.M22 = matrix.M22;
		}

		/// <summary>
		/// Obtains the transpose matrix.
		/// </summary>
		/// <returns>Transpose matrix.</returns>
		public Matrix3 Transpose()
		{
			return new Matrix3(this.M00, this.M10, this.M20,
							this.M01, this.M11, this.M21,
							this.M02, this.M12, this.M22);
		}

		/// <summary>
		/// Gets the rotation matrix from the normal vector (extrusion direction) of an entity.
		/// </summary>
		/// <param name="zAxis">Normal vector.</param>
		/// <returns>Rotation matrix.</returns>
		public static Matrix3 ArbitraryAxis(XYZ zAxis)
		{
			zAxis.Normalize();

			if (zAxis.Equals(XYZ.AxisZ))
			{
				return Matrix3.Identity;
			}

			XYZ wY = XYZ.AxisY;
			XYZ wZ = XYZ.AxisZ;
			XYZ aX;

			if ((Math.Abs(zAxis.X) < 1 / 64.0) && (Math.Abs(zAxis.Y) < 1 / 64.0))
			{
				aX = XYZ.Cross(wY, zAxis);
			}
			else
			{
				aX = XYZ.Cross(wZ, zAxis);
			}

			aX.Normalize();

			XYZ aY = XYZ.Cross(zAxis, aX);
			aY.Normalize();

			return new Matrix3(aX.X, aY.X, zAxis.X, aX.Y, aY.Y, zAxis.Y, aX.Z, aY.Z, zAxis.Z);
		}

		/// <summary>
		/// Builds a rotation matrix for a rotation around the z-axis.
		/// </summary>
		/// <param name="angle">The counter-clockwise angle in radians.</param>
		/// <returns>The resulting Matrix3 instance.</returns>
		/// <remarks>Matrix3 adopts the convention of using column vectors to represent a transformation matrix.</remarks>
		public static Matrix3 RotationZ(double angle)
		{
			double cos = Math.Cos(angle);
			double sin = Math.Sin(angle);
			return new Matrix3(cos, -sin, 0.0,
							   sin, cos, 0.0,
							   0.0, 0.0, 1.0);
		}

		/// <inheritdoc/>
		public override string ToString()
		{
			string separator = Thread.CurrentThread.CurrentCulture.TextInfo.ListSeparator;
			StringBuilder s = new StringBuilder();
			s.Append(string.Format("|{0}{3} {1}{3} {2}|" + Environment.NewLine, this.M00, this.M01, this.M02, separator));
			s.Append(string.Format("|{0}{3} {1}{3} {2}|" + Environment.NewLine, this.M10, this.M11, this.M12, separator));
			s.Append(string.Format("|{0}{3} {1}{3} {2}|", this.M20, this.M21, this.M22, separator));
			return s.ToString();
		}
	}
}