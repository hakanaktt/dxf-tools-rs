using CSUtilities.Tests.Mock;
using Xunit;

namespace CSUtilities.Tests
{
	public class AppDomainUtilsTests
	{
		[Fact]
		public void GetTypesOfInterfaceTest()
		{
			var types = AppDomainUtils.GetTypesOfInterface<IMockInterface>();

			Assert.NotEmpty(types);
			Assert.Contains(typeof(Mock01), types);
		}

		[Fact]
		public void GetTypesWithAttributeTest()
		{
			var types = AppDomainUtils.GetTypesWithAttribute<MyMockAttribute>();

			Assert.NotEmpty(types);
			Assert.Contains(typeof(Mock02), types);
		}
	}
}