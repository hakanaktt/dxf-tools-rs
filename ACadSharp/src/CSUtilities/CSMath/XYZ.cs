using System;

namespace CSMath
{
	public partial struct XYZ : IVector, IEquatable<XYZ>
	{
		/// <inheritdoc/>
		public uint Dimension { get { return 3; } }

		/// <summary>
		/// Specifies the X-value of the vector component
		/// </summary>
		public double X { get; set; }

		/// <summary>
		/// Specifies the Y-value of the vector component
		/// </summary>
		public double Y { get; set; }

		/// <summary>
		/// Specifies the Z-value of the vector component
		/// </summary>
		public double Z { get; set; }

		public static readonly XYZ AxisX = new XYZ(1, 0, 0);

		public static readonly XYZ AxisY = new XYZ(0, 1, 0);

		public static readonly XYZ AxisZ = new XYZ(0, 0, 1);

		public static readonly XYZ NaN = new XYZ(double.NaN);

		public static readonly XYZ Zero = new XYZ(0, 0, 0);

		/// <summary>
		/// Constructor with the coordinate components
		/// </summary>
		/// <param name="x">Value of the X-coordinate</param>
		/// <param name="y">Value of the Y-coordinate</param>
		/// <param name="z">Value of the Z-coordinate</param>
		public XYZ(double x, double y, double z)
		{
			X = x;
			Y = y;
			Z = z;
		}

		/// <summary>
		/// Constructs a vector whose elements are all the single specified value.
		/// </summary>
		/// <param name="value">The element to fill the vector with.</param>
		public XYZ(double value) : this(value, value, value) { }

		[Obsolete("Deprecated")]
		public XYZ(double[] components) : this(components[0], components[1], components[2]) { }

		[Obsolete("Deprecated")]
		public static XYZ CreateFrom(double[] arr, int offset)
		{
			double[] values = new double[3];
			for (int i = offset; i < arr.Length && i < values.Length + offset; i++)
			{
				values[i] = (double)arr[i];
			}

			return new XYZ(values);
		}

		/// <summary>
		/// Computes the cross product of two coordinates.
		/// </summary>
		/// <param name="xyz1">The first coordinate.</param>
		/// <param name="xyz2">The second coordinate.</param>
		/// <returns>The cross product.</returns>
		public static XYZ Cross(XYZ xyz1, XYZ xyz2)
		{
			return new XYZ(
				xyz1.Y * xyz2.Z - xyz1.Z * xyz2.Y,
				xyz1.Z * xyz2.X - xyz1.X * xyz2.Z,
				xyz1.X * xyz2.Y - xyz1.Y * xyz2.X);
		}

		public static XYZ FindNormal(XYZ point1, XYZ point2, XYZ point3)
		{
			XYZ a = point2.Subtract(point1);
			XYZ b = point3.Subtract(point1);

			// N = Cross(a, b)
			XYZ n = XYZ.Cross(a, b);
			XYZ normal = n.Normalize();

			return normal;
		}

		/// <inheritdoc/>
		public override bool Equals(object? obj)
		{
			if (!(obj is XYZ other))
				return false;

			return this.Equals(other);
		}

		/// <summary>
		/// Indicates whether this instance and a specified object are equal with in a specific precision.
		/// </summary>
		/// <param name="other"></param>
		/// <param name="digits">number of decimals</param>
		/// <returns></returns>
		public bool Equals(XYZ other, int digits)
		{
			return other.IsEqual(this, digits);
		}

		/// <inheritdoc/>
		public bool Equals(XYZ other)
		{
			return this.X == other.X && this.Y == other.Y && this.Z == other.Z;
		}

		/// <summary>
		/// Calculates the angle, in radians, between this vector and the specified vector.
		/// </summary>
		/// <remarks>If either vector has zero length, the result may not be meaningful. The method returns 0 if the
		/// vectors are parallel and point in the same direction, and π if they are parallel and point in opposite
		/// directions.</remarks>
		/// <param name="dir">The vector to which the angle is measured.</param>
		/// <returns>The angle, in radians, between this vector and <paramref name="dir"/>. The value is in the range [0, π].</returns>
		public double GetAngle(XYZ dir)
		{
			double t = this.Dot(dir) / Math.Sqrt(this.GetLengthSquared() * dir.GetLengthSquared());
			if (MathHelper.IsAlmostZero(Math.Abs(t) - 1.0d))
			{
				if (!((double)t > 0.0))
				{
					return Math.PI;
				}
				return 0.0;
			}
			return Math.Acos((double)t);
		}

		/// <summary>
		/// Calculates the signed angle, in radians, between this vector and the specified direction vector, using the given
		/// normal to determine the sign.
		/// </summary>
		/// <remarks>The returned angle is measured in the plane defined by this vector and <paramref name="dir"/>,
		/// with the sign determined by the right-hand rule using <paramref name="normal"/>. If the vectors are parallel, the
		/// angle is 0 or π depending on their direction.</remarks>
		/// <param name="dir">The direction vector to which the angle is measured.</param>
		/// <param name="normal">The normal vector that defines the orientation of the plane in which the angle is measured. The sign of the angle
		/// is determined based on the direction of this normal.</param>
		/// <returns>The signed angle, in radians, between this vector and <paramref name="dir"/>. The value is in the range [0, 2π].</returns>
		public double GetAngle2(XYZ dir, XYZ normal)
		{
			double t = this.Dot(dir) / Math.Sqrt(this.GetLengthSquared() * dir.GetLengthSquared());
			if (MathHelper.IsAlmostZero(Math.Abs(t) - 1.0d))
			{
				if (!((double)t > 0.0))
				{
					return Math.PI;
				}
				return 0.0;
			}

			double angle = this.GetAngle(dir);
			XYZ vector = Cross(this, dir);
			if (!(vector.Dot(normal) > 0.0))
			{
				return MathHelper.TwoPI - angle;
			}

			return angle;
		}

		/// <inheritdoc/>
		public override int GetHashCode()
		{
			return this.X.GetHashCode() ^ this.Y.GetHashCode() ^ this.Z.GetHashCode();
		}

		/// <inheritdoc/>
		public override string ToString()
		{
			return $"{X},{Y},{Z}";
		}

		/// <summary>
		/// Converts the numeric value of this instance to its equivalent string representation
		/// using the specified culture-specific format information.
		/// </summary>
		/// <param name="cultureInfo">An object that supplies culture-specific formatting information.</param>
		/// <returns>The string representation of the value of this instance as specified by provider.</returns>
		public string ToString(IFormatProvider? cultureInfo)
		{
			return $"{X.ToString(cultureInfo)},{Y.ToString(cultureInfo)},{Z.ToString(cultureInfo)}";
		}

		/// <inheritdoc/>
		public double this[int index]
		{
			get
			{
				switch (index)
				{
					case 0:
						return X;
					case 1:
						return Y;
					case 2:
						return Z;
					default:
						throw new IndexOutOfRangeException($"The index must be between 0 and {this.Dimension}.");
				}
			}
			set
			{
				switch (index)
				{
					case 0:
						X = value;
						break;
					case 1:
						Y = value;
						break;
					case 2:
						Z = value;
						break;
					default:
						throw new IndexOutOfRangeException($"The index must be between 0 and {this.Dimension}.");
				}
			}
		}
	}
}