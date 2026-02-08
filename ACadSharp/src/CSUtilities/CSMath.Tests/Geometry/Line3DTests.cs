using CSMath.Geometry;
using Xunit;

namespace CSMath.Tests.Geometry
{
	public class Line3DTests
	{
		[Fact]
		public void CreateLineTest()
		{
			//Line3D line = LineExtensions.CreateFromPoints<Line3D, XYZ>(new XYZ(), new XYZ(1, 1));
		}

		[Fact]
		public void FindIntersectionTest()
		{
			Line3D line1 = new Line3D(XYZ.Zero, XYZ.AxisX);
			Line3D line2 = new Line3D(new XYZ(1, 1, 0), XYZ.AxisY);

			XYZ intersection = line1.FindIntersection(line2);

			Assert.True(intersection.Equals(new XYZ(1, 0, 0)));
		}

		[Fact]
		public void IsPointOnLineTest()
		{
			Line3D line = new Line3D();

			line.IsPointOnLine(new XYZ());
		}
	}
}