using System;

namespace CSUtilities.Tests.Mock
{
	[System.AttributeUsage(AttributeTargets.All, Inherited = false, AllowMultiple = true)]
	internal sealed class MyMockAttribute : Attribute
	{
		// This is a named argument
		public int NamedInt { get; set; }

		public string PositionalString
		{
			get { return positionalString; }
		}

		// See the attribute guidelines at
		//  http://go.microsoft.com/fwlink/?LinkId=85236
		private readonly string positionalString;

		// This is a positional argument
		public MyMockAttribute(string positionalString)
		{
			this.positionalString = positionalString;
		}
	}
}