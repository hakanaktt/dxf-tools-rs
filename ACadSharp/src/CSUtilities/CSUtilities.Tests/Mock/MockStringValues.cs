using CSUtilities.Attributes;

namespace CSUtilities.Tests.Mock
{
	public enum MockStringValues
	{
		[StringValue("Undefined value for enum")]
		Undefined = 0,

		[StringValue("Enum String Value 1")]
		Value1 = 1,

		[StringValue("Enum String Value 2")]
		Value2,

		NoAttribute,
	}
}