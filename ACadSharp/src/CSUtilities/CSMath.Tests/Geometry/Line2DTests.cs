using CSMath.Geometry;
using Xunit;

namespace CSMath.Tests.Geometry
{
	public class Line2DTests
	{
		[Fact]
		public void CreateLineTest()
		{
			//Line2D line = LineExtensions.CreateFromPoints<Line2D, XY>(new XY(), new XY(1, 1));
		}

		[Fact]
		public void IsPointOnLineTest()
		{
			Line2D line = new Line2D();

			line.IsPointOnLine(new XY());
		}

		[Fact]
		public void FindIntersectionTest()
		{
			Line2D line1 = new Line2D(XY.Zero, XY.AxisX);
			Line2D line2 = new Line2D(new XY(1, 1), XY.AxisY);
			XY intersection = line1.FindIntersection(line2);

			Assert.True(intersection.Equals(new XY(1, 0)));
		}
	}
}
