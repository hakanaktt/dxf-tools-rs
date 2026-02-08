using System;

namespace CSMath.Geometry
{
	public static class LineExtensions
	{
		public static T CreateFromPoints<T, R>(R pt1, R pt2)
			where T : ILine<R>
			where R : IVector, new()
		{
			return (T)Activator.CreateInstance(typeof(T), pt1, pt2.Subtract(pt1));
		}

		/// <summary>
		/// Determines whether the specified point is on the line, or not.
		/// </summary>
		/// <param name="line"></param>
		/// <param name="point"></param>
		/// <returns></returns>
		public static bool IsPointOnLine<T>(this ILine<T> line, T point)
			where T : IVector
		{
			double lambda = 0;
			for (int i = 0; i < point.Dimension; ++i)
			{
				var value = (point[i] - line.Origin[i]) / line.Direction[i];
				if (i != 0 && (value != lambda))
				{
					return false;
				}

				lambda = value;
			}

			return true;
		}

		/// <summary>
		/// Tries to find the intersection between 2 lines.
		/// </summary>
		/// <typeparam name="T"></typeparam>
		/// <typeparam name="R"></typeparam>
		/// <param name="line1"></param>
		/// <param name="line2"></param>
		/// <param name="intersection"></param>
		/// <returns>True if the intersection is found.</returns>
		public static bool TryFindIntersection<T, R>(this T line1, T line2, out R intersection)
			where T : ILine<R>
			where R : IVector
		{
			intersection = line1.FindIntersection(line2);

			return !intersection.IsNaN();
		}
	}
}