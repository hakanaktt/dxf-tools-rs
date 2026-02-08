using Xunit;

namespace CSMath.Tests
{
	public class BoundingBoxTests
	{
		[Theory]
		[InlineData(0, 0, 0, true)]
		[InlineData(11, 0, 0, false)]
		[InlineData(11, 0, -11, false)]
		[InlineData(5, 0, 5, true)]
		[InlineData(10, 10, 10, true)]
		public void IsInPointTest(double x, double y, double z, bool isIn)
		{
			BoundingBox box = new BoundingBox(-10, -10, -10, 10, 10, 10);

			XYZ xyz = new XYZ(x, y, z);
			Assert.Equal(isIn, box.IsIn(xyz));
		}

		[Theory]
		[InlineData(-10, -10, -10, 10, 10, 10, true, true)]
		[InlineData(-5, -5, -5, 5, 5, 5, true, true)]
		[InlineData(-5, -5, -5, 5, 50, 50, false, true)]
		[InlineData(-50, -60, -50, -11, -11, -11, false, false)]
		public void IsInBoxTest(double minX, double minY, double minZ, double maxX, double maxY, double maxZ, bool isIn, bool partialInside)
		{
			BoundingBox box = new BoundingBox(-10, -10, -10, 10, 10, 10);

			BoundingBox test = new BoundingBox(minX, minY, minZ, maxX, maxY, maxZ);
			Assert.Equal(isIn, box.IsIn(test, out bool partialyIn));
			Assert.Equal(partialInside, partialyIn);
		}
	}
}
