using System;
using System.Collections.Generic;

namespace CSMath.Geometry
{
	public static class CurveExtensions
	{
		/// <summary>
		/// Find a coordinate in a curve.
		/// </summary>
		/// <param name="angle"></param>
		/// <param name="center"></param>
		/// <param name="normal"></param>
		/// <param name="startPoint"></param>
		/// <param name="ratio"></param>
		/// <returns></returns>
		public static XYZ PolarCoordinate(double angle, XYZ center, XYZ normal, XYZ startPoint, double ratio = 1)
		{
			XYZ prep = XYZ.Cross(normal, startPoint);
			return (Math.Cos(angle) * startPoint + (double)ratio * Math.Sin(angle) * prep) + center;
		}

		/// <summary>
		/// Converts a curve into a list of vertexes.
		/// </summary>
		/// <param name="precision"></param>
		/// <param name="center"></param>
		/// <param name="startAngle"></param>
		/// <param name="endAngle"></param>
		/// <param name="radius"></param>
		/// <param name="normal"></param>
		/// <returns></returns>
		/// <exception cref="ArgumentOutOfRangeException"></exception>
		public static List<XYZ> PolygonalVertexes(int precision, XYZ center, double startAngle, double endAngle, double radius, XYZ normal)
		{
			if (precision < 2)
			{
				throw new ArgumentOutOfRangeException(nameof(precision), precision, "The precision must be equal or greater than two.");
			}

			List<XYZ> points = new();

			double start = startAngle;
			double end = endAngle;
			if (end <= startAngle)
			{
				end += MathHelper.TwoPI;
			}

			var t = Matrix4.GetArbitraryAxis(normal);

			var sweep = start - end;
			var denominator = MathHelper.IsEqual(sweep, MathHelper.TwoPI) ? precision : precision - 1;
			double delta = (end - start) / denominator;
			for (int i = 0; i < precision; i++, start += delta)
			{
				var pt = new XYZ(MathHelper.Cos(start), MathHelper.Sin(start), 0.0);
				pt = center + radius * pt;
				points.Add(t * pt);
			}

			return points;
		}

		/// <summary>
		/// Converts a curve into a list of vertexes.
		/// </summary>
		/// <param name="length"></param>
		/// <param name="center"></param>
		/// <param name="startAngle"></param>
		/// <param name="endAngle"></param>
		/// <param name="radius"></param>
		/// <param name="normal"></param>
		/// <returns></returns>
		public static List<XYZ> PolygonalVertexes(double length, XYZ center, double startAngle, double endAngle, double radius, XYZ normal)
		{
			var longitude = 2 * Math.PI * radius;

			int nsegments = (int)Math.Ceiling(longitude / length);

			if (nsegments < 2)
			{
				nsegments = 3;
			}

			return PolygonalVertexes(nsegments, center, startAngle, endAngle, radius, normal);
		}

		/// <summary>
		/// Converts an elliptical curve into a list of vertexes.
		/// </summary>
		/// <param name="precision"></param>
		/// <param name="center"></param>
		/// <param name="startAngle"></param>
		/// <param name="endAngle"></param>
		/// <param name="normal"></param>
		/// <param name="majorAxisPoint"></param>
		/// <param name="ratio"></param>
		/// <returns></returns>
		/// <exception cref="ArgumentOutOfRangeException"></exception>
		public static List<XYZ> PolygonalVertexes(int precision, XYZ center, double startAngle, double endAngle, XYZ normal, XYZ majorAxisPoint, double ratio = 1)
		{
			if (precision < 2)
			{
				throw new ArgumentOutOfRangeException(nameof(precision), precision, "The precision must be equal or greater than two.");
			}

			List<XYZ> points = new();

			double start = startAngle;
			double end = endAngle;
			if (end <= start)
			{
				end += MathHelper.TwoPI;
			}

			XYZ relative = majorAxisPoint - center;
			XYZ perp = XYZ.Cross(normal, relative);

			var sweep = start - end;
			var denominator = MathHelper.IsEqual(sweep, MathHelper.TwoPI) ? precision : precision - 1;
			double delta = (end - start) / denominator;
			for (int i = 0; i < precision; i++, start += delta)
			{
				points.Add(MathHelper.Cos(start) * relative
					+ (double)ratio * MathHelper.Sin(start) * perp + center);
			}

			return points;
		}
	}
}