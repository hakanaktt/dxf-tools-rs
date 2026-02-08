using System;

namespace CSUtilities
{
#if PUBLIC
	public
#else
	internal
#endif
		static class Try
	{
		/// <summary>
		/// Executes the specified action and returns a value indicating whether the operation completed successfully.
		/// </summary>
		/// <param name="action">The action to execute.</param>
		/// <returns>true if the action completed without throwing an exception; otherwise, false.</returns>
		public static bool Do(Action action)
		{
			return Do(action, out _);
		}

		/// <summary>
		/// Executes the specified action and captures any exception that occurs during its execution.
		/// </summary>
		/// <remarks>Use this method to execute an action and handle exceptions without using a try-catch block in the
		/// calling code. This method is thread-safe.</remarks>
		/// <param name="action">The action to execute.</param>
		/// <param name="exception">When this method returns, contains the exception that was thrown by the action, if any; otherwise, null.</param>
		/// <returns>true if the action completes without throwing an exception; otherwise, false.</returns>
		public static bool Do(Action action, out Exception exception)
		{
			exception = null;

			try
			{
				action();
				return true;
			}
			catch (Exception ex)
			{
				exception = ex;
				return false;
			}
		}
	}
}