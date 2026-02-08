using CSUtilities.Extensions;
using System;
using Xunit;

namespace CSUtilities.Tests.Extensions
{
	public class ObjectExtensionsTests
	{
		[Fact]
		public void ThrowIfTest()
		{
			int zero = 0;

			Assert.Throws<ArgumentOutOfRangeException>(() => zero.ThrowIf<int, ArgumentOutOfRangeException>((value) =>
				{
					return value == 0;
				}));
		}

		[Theory]
		[InlineData(5, 0, 10)]
		[InlineData(5, 5, 10)]
		[InlineData(5, 5, 10, false, true)]
		[InlineData(10, 0, 10)]
		[InlineData(10, 0, 10, false, true)]
		[InlineData(-5, -10, 10)]
		[InlineData(-20, -10, 10, false, true)]
		[InlineData(20, -10, 10, false, true)]
		public void InRangeTest(int value, int min, int max, bool inclusive = true, bool throws = false)
		{
			if (throws)
			{
				Assert.Throws<ArgumentOutOfRangeException>(() => ObjectExtensions.InRange(value, min, max, inclusive));
			}
			else
			{
				ObjectExtensions.InRange(value, min, max, inclusive);
			}
		}

		[Theory]
		[InlineData(5, 0)]
		[InlineData(5, 5)]
		[InlineData(5, 5, false, true)]
		[InlineData(10, 10)]
		[InlineData(10, 10, false, true)]
		[InlineData(50, 10)]
		[InlineData(-20, 10, false, true)]
		[InlineData(20, 10, false, false)]
		public void GreaterThanTest(int value, int min, bool inclusive = true, bool throws = false)
		{
			if (throws)
			{
				Assert.Throws<ArgumentOutOfRangeException>(() => ObjectExtensions.GreaterThan(value, min, inclusive));
			}
			else
			{
				ObjectExtensions.GreaterThan(value, min, inclusive);
			}
		}
	}
}
