using System;

namespace CSUtilities.Tests.Mock
{
	[Flags]
	public enum MockFlags
	{
		None = 0,

		Flag1 = 1,

		Flag2 = 2,

		Flag3 = 4,

		All = Flag1 | Flag2 | Flag3
	}
}