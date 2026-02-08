using CSUtilities.Extensions;
using CSUtilities.Tests.Mock;
using System;
using Xunit;

namespace CSUtilities.Tests.Extensions
{
	public class EnumExtensionsTests
	{
		[Fact]
		public void AddFlagTest()
		{
			MockFlags flags = MockFlags.None;

			flags.AddFlag(MockFlags.Flag1);
			Assert.Equal(MockFlags.Flag1, flags);
			flags.AddFlag(MockFlags.Flag2);
			Assert.Equal(MockFlags.Flag1 | MockFlags.Flag2, flags);
			flags.AddFlag(MockFlags.Flag3);
			Assert.Equal(MockFlags.All, flags);
			flags.AddFlag(MockFlags.Flag1);
			Assert.Equal(MockFlags.All, flags);
		}

		[Fact]
		public void GetStringValueTest()
		{
			Assert.Equal("Undefined value for enum", MockStringValues.Undefined.GetStringValue());
			Assert.Equal("Enum String Value 1", MockStringValues.Value1.GetStringValue());
			Assert.Equal("Enum String Value 2", MockStringValues.Value2.GetStringValue());
			Assert.Null(MockStringValues.NoAttribute.GetStringValue());
		}

		[Fact]
		public void ParseTest()
		{
			Assert.Equal(MockStringValues.Value1, EnumExtensions.Parse<MockStringValues>("Value1"));
			Assert.Equal(MockStringValues.Value1, EnumExtensions.Parse<MockStringValues>("value1", true));
			Assert.Throws<ArgumentException>(() => EnumExtensions.Parse<MockStringValues>("value1", false));
		}

		[Fact]
		public void RemoveFlagTest()
		{
			MockFlags flags = MockFlags.All;

			flags.RemoveFlag(MockFlags.Flag1);
			Assert.Equal(MockFlags.Flag2 | MockFlags.Flag3, flags);
			flags.RemoveFlag(MockFlags.Flag2);
			Assert.Equal(MockFlags.Flag3, flags);
			flags.RemoveFlag(MockFlags.Flag3);
			Assert.Equal(MockFlags.None, flags);
		}

		[Fact]
		public void TryParseTest()
		{
			MockStringValues result = MockStringValues.Undefined;
			Assert.True(EnumExtensions.TryParse("Value1", out result));
			Assert.Equal(MockStringValues.Value1, result);

			result = MockStringValues.Undefined;
			Assert.True(EnumExtensions.TryParse<MockStringValues>("value1", out result, true));
			Assert.Equal(MockStringValues.Value1, result);

			Assert.False(EnumExtensions.TryParse<MockStringValues>("value1", out result, false));
			Assert.Equal(default(MockStringValues), result);
		}
	}
}