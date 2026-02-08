using System;
using System.Collections.Generic;

namespace CSMath
{
	/// <summary>
	/// Bounding box representation aligned to XYZ axis.
	/// </summary>
	public struct BoundingBox
	{
		/// <summary>
		/// Instance of a null bounding box.
		/// </summary>
		public static readonly BoundingBox Null = new BoundingBox(BoundingBoxExtent.Null);

		/// <summary>
		/// Instance of an infinite bounding box.
		/// </summary>
		public static readonly BoundingBox Infinite = new BoundingBox(BoundingBoxExtent.Infinite);

		public BoundingBoxExtent Extent { get; }

		/// <summary>
		/// Get the min corner of the bounding box.
		/// </summary>
		public XYZ Min { get; set; }

		/// <summary>
		/// Get the max corner of the bounding box.
		/// </summary>
		public XYZ Max { get; set; }

		/// <summary>
		/// Center of the box.
		/// </summary>
		public XYZ Center
		{
			get
			{
				return this.Min + (this.Max - this.Min) * 0.5;
			}
		}

		/// <summary>
		/// Gets the length of the bounding box along the X-axis.
		/// </summary>
		public double LengthX
		{
			get
			{
				return Math.Abs(this.Max.X - this.Min.X);
			}
		}

		/// <summary>
		/// Gets the length of the bounding box along the Y-axis.
		/// </summary>
		public double LengthY
		{
			get
			{
				return Math.Abs(this.Max.Y - this.Min.Y);
			}
		}

		/// <summary>
		/// Gets the length of the bounding box along the Z-axis.
		/// </summary>
		public double LengthZ
		{
			get
			{
				return Math.Abs(this.Max.Z - this.Min.Z);
			}
		}

		[Obsolete("Use LengthX instead.")]
		public double Width
		{
			get
			{
				return Math.Abs(this.Max.X - this.Min.X);
			}
		}

		[Obsolete("Use LengthY instead.")]
		public double Height
		{
			get
			{
				return Math.Abs(this.Max.Y - this.Min.Y);
			}
		}

		private BoundingBox(BoundingBoxExtent extent)
		{
			this.Extent = extent;
			switch (extent)
			{
				case BoundingBoxExtent.Null:
					this.Max = new XYZ(double.NaN);
					this.Min = new XYZ(double.NaN);
					break;
				case BoundingBoxExtent.Infinite:
					this.Min = new XYZ(double.NegativeInfinity);
					this.Max = new XYZ(double.PositiveInfinity);
					break;
				case BoundingBoxExtent.Finite:
				case BoundingBoxExtent.Point:
				default:
					break;
			}
		}

		/// <summary>
		/// Initializes a new instance of the BoundingBox class that represents a single point.
		/// </summary>
		/// <param name="point">The point to use as both the minimum and maximum bounds of the bounding box.</param>
		public BoundingBox(XYZ point)
		{
			this.Extent = BoundingBoxExtent.Point;
			this.Min = point;
			this.Max = point;
		}

		/// <summary>
		/// Initializes a new instance of the BoundingBox class that represents the axis-aligned box defined by the specified
		/// minimum and maximum coordinates.
		/// </summary>
		/// <remarks>If the specified minimum and maximum points are equal, the bounding box represents a single point.
		/// The constructor automatically assigns the smaller of each coordinate to Min and the larger to Max, regardless of the
		/// order of the input parameters.</remarks>
		/// <param name="min">The point representing the minimum X, Y, and Z coordinates of the bounding box.</param>
		/// <param name="max">The point representing the maximum X, Y, and Z coordinates of the bounding box.</param>
		public BoundingBox(XYZ min, XYZ max)
		{
			if (min == max)
			{
				this.Extent = BoundingBoxExtent.Point;
			}
			else
			{
				this.Extent = BoundingBoxExtent.Finite;
			}

			this.Min = new XYZ(System.Math.Min(min.X, max.X), System.Math.Min(min.Y, max.Y), System.Math.Min(min.Z, max.Z));
			this.Max = new XYZ(System.Math.Max(min.X, max.X), System.Math.Max(min.Y, max.Y), System.Math.Max(min.Z, max.Z));
		}

		/// <summary>
		/// Initializes a new instance of the BoundingBox structure using the specified minimum and maximum X, Y, and Z
		/// coordinates.
		/// </summary>
		/// <param name="minX">The minimum X coordinate of the bounding box.</param>
		/// <param name="minY">The minimum Y coordinate of the bounding box.</param>
		/// <param name="minZ">The minimum Z coordinate of the bounding box.</param>
		/// <param name="maxX">The maximum X coordinate of the bounding box.</param>
		/// <param name="maxY">The maximum Y coordinate of the bounding box.</param>
		/// <param name="maxZ">The maximum Z coordinate of the bounding box.</param>
		public BoundingBox(double minX, double minY, double minZ, double maxX, double maxY, double maxZ)
		{
			this = new BoundingBox(new XYZ(minX, minY, minZ), new XYZ(maxX, maxY, maxZ));
		}

		/// <summary>
		/// Returns a new bounding box translated by the specified vector.
		/// </summary>
		/// <param name="xyz">The vector by which to translate the bounding box. Each component is added to the corresponding coordinates of the
		/// bounding box's minimum and maximum points.</param>
		/// <returns>A new BoundingBox instance that is the result of translating the current bounding box by the specified vector.</returns>
		public BoundingBox Move(XYZ xyz)
		{
			return new BoundingBox(this.Min + xyz, this.Max + xyz);
		}

		/// <summary>
		/// Merge 2 boxes into the common one.
		/// </summary>
		/// <param name="box"></param>
		/// <returns>The merged box.</returns>
		public BoundingBox Merge(BoundingBox box)
		{
			if (this.Extent == BoundingBoxExtent.Infinite
				|| box.Extent == BoundingBoxExtent.Infinite)
			{
				return BoundingBox.Infinite;
			}
			else if (this.Extent == BoundingBoxExtent.Null)
			{
				return box;
			}
			else if (box.Extent == BoundingBoxExtent.Null)
			{
				return this;
			}

			var min = new XYZ(
				System.Math.Min(this.Min.X, box.Min.X),
				System.Math.Min(this.Min.Y, box.Min.Y),
				System.Math.Min(this.Min.Z, box.Min.Z));
			var max = new XYZ(
				System.Math.Max(this.Max.X, box.Max.X),
				System.Math.Max(this.Max.Y, box.Max.Y),
				System.Math.Max(this.Max.Z, box.Max.Z));

			return new BoundingBox(min, max);
		}

		/// <summary>
		/// Checks if the given box is in the bounds of this box.
		/// </summary>
		/// <param name="box"></param>
		/// <returns></returns>
		public bool IsIn(BoundingBox box)
		{
			return this.IsIn(box, out _);
		}

		/// <summary>
		/// Checks if the given box is in the bounds of this box.
		/// </summary>
		/// <param name="box"></param>
		/// <param name="partialIn">Flag to notify that on part of the box is inside but not completely.</param>
		/// <returns></returns>
		public bool IsIn(BoundingBox box, out bool partialIn)
		{
			bool min = this.IsIn(box.Min);
			bool max = this.IsIn(box.Max);

			partialIn = min || max;

			return min && max;
		}

		/// <summary>
		/// Checks if the point is in the bounds of the box.
		/// </summary>
		/// <param name="point"></param>
		/// <returns></returns>
		public bool IsIn(XYZ point)
		{
			if (this.Min.X > point.X || this.Min.Y > point.Y || this.Min.Z > point.Z)
			{
				return false;
			}

			if (this.Max.X < point.X || this.Max.Y < point.Y || this.Max.Z < point.Z)
			{
				return false;
			}

			return true;
		}

		/// <summary>
		/// Merge Multiple boxes into the common one.
		/// </summary>
		/// <param name="boxes"></param>
		/// <returns>The merged box.</returns>
		public static BoundingBox Merge(IEnumerable<BoundingBox> boxes)
		{
			BoundingBox b = BoundingBox.Null;

			foreach (var box in boxes)
			{
				b = b.Merge(box);
			}

			return b;
		}

		/// <summary>
		/// Create a bounding box from a collection of points.
		/// </summary>
		/// <param name="points"></param>
		/// <returns></returns>
		public static BoundingBox FromPoints(IEnumerable<XYZ> points)
		{
			BoundingBox boundingBox = Null;

			foreach (var point in points)
			{
				boundingBox = boundingBox.Merge(new BoundingBox(point));
			}

			return boundingBox;
		}
	}
}