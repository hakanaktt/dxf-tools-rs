using Xunit;

namespace CSMath.Tests
{
	public class TransformTests
	{
		private CSMathRandom _random = new CSMathRandom();

		[Fact()]
		public void TranslationTest()
		{
			XYZ translation = _random.Next<XYZ>();
			XYZ scale = _random.Next<XYZ>();
			XYZ rotation = _random.Next<XYZ>();

			Transform transform = new Transform(translation, scale, rotation);

			Assert.Equal(translation, transform.Translation);
		}

		[Fact()]
		public void ApplyTranslationTest()
		{
			XYZ xyz = XYZ.Zero;

			XYZ translation = XYZ.Zero;
			XYZ scale = new XYZ(1, 1, 1);
			XYZ rotation = XYZ.Zero;

			XYZ expected = xyz + translation;

			Transform transform = new Transform(translation, scale, rotation);
			XYZ result = transform.ApplyTransform(xyz);

			AssertUtils.AreEqual(expected, result);
		}

		[Fact()]
		public void ScaleTest()
		{
			XYZ translation = _random.Next<XYZ>();
			XYZ scale = _random.Next<XYZ>();
			XYZ rotation = _random.Next<XYZ>();

			Transform transform = new Transform(translation, scale, rotation);

			AssertUtils.AreEqual(scale, transform.Scale, "Scale");
		}

		[Fact()]
		public void ApplyScaleTest()
		{
			XYZ xyz = new XYZ(1, 1, 1);

			XYZ translation = XYZ.Zero;
			XYZ scale = _random.Next<XYZ>();
			XYZ rotation = XYZ.Zero;

			XYZ expected = xyz * scale;

			Transform transform = new Transform(translation, scale, rotation);
			XYZ result = transform.ApplyTransform(xyz);

			AssertUtils.AreEqual(expected, result);
		}

		[Fact]
		public void RotationTest()
		{
			XYZ translation = _random.Next<XYZ>();
			XYZ scale = _random.Next<XYZ>();
			XYZ rotation = _random.Next<XYZ>();

			Transform transform = new Transform(translation, scale, rotation);

			Assert.Equal(rotation, transform.EulerRotation);
		}

		[Fact()]
		public void ApplyRotationTest()
		{
			XYZ xyz = XYZ.AxisX;

			XYZ translation = XYZ.Zero;
			XYZ scale = new XYZ(1, 1, 1);
			XYZ rotation = new XYZ(0, 0, MathHelper.DegToRad(90));

			XYZ expected = new XYZ(0, 1, 0);

			Transform transform = new Transform(translation, scale, rotation);
			XYZ result = transform.ApplyTransform(xyz);

			AssertUtils.AreEqual(expected, result);
		}

		[Fact()]
		public void DecomposeTest()
		{
			XYZ translation = _random.Next<XYZ>();
			XYZ scale = _random.Next<XYZ>();
			XYZ rotation = new XYZ(90, 0, 0);

			Transform transform = new Transform(translation, scale, rotation);

			transform.TryDecompose(out XYZ t, out XYZ s, out Quaternion r);

			AssertUtils.AreEqual(transform.Translation, t, "translation");
			AssertUtils.AreEqual(transform.Scale, s, "scale");
			AssertUtils.AreEqual(transform.Quaternion, r, "rotation");
		}
	}
}
